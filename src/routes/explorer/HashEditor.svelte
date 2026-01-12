<script>
  import { invoke } from "@tauri-apps/api/core";
  import { activeConfig } from "$lib/stores.js";
  import { createEventDispatcher } from "svelte";

  let { selectedKey = "", selectedDb = 0 } = $props();

  const dispatch = createEventDispatcher();

  // State
  let fields = $state([]);
  let fieldValues = $state({}); // Cache field values for table display
  let selectedField = $state("");
  let editingFieldName = $state(""); // Editable field name
  let fieldValue = $state("");
  let originalFieldName = $state(""); // Track original for change detection
  let originalFieldValue = $state(""); // Track original for change detection
  let cursor = $state(0);
  let isLoading = $state(false);
  let isSaving = $state(false);
  let hasMore = $state(true);

  // Filter states
  let showFilter = $state(false);
  let searchTarget = $state("both"); // "field", "value", "both"
  let searchCondition = $state("contains"); // "contains", "begins", "ends", "glob"
  let searchText = $state("");

  // Pagination states
  let currentPage = $state(1);
  let itemsPerPage = $state(100);

  // Computed: has changes?
  let hasChanges = $derived(
    editingFieldName !== originalFieldName || fieldValue !== originalFieldValue
  );

  // Computed: filtered fields
  let filteredFields = $derived(
    !searchText.trim()
      ? fields
      : fields.filter((field) => {
          const fieldName = field.toLowerCase();
          const fieldVal = (fieldValues[field] || "").toLowerCase();
          const search = searchText.toLowerCase();

          let matchField = false;
          let matchValue = false;

          if (searchTarget === "field" || searchTarget === "both") {
            matchField = applyCondition(fieldName, search, searchCondition);
          }
          if (searchTarget === "value" || searchTarget === "both") {
            matchValue = applyCondition(fieldVal, search, searchCondition);
          }

          return searchTarget === "both"
            ? matchField || matchValue
            : matchField || matchValue;
        })
  );

  // Computed: total pages
  let totalPages = $derived(Math.ceil(filteredFields.length / itemsPerPage));

  // Computed: paginated fields
  let paginatedFields = $derived(
    (() => {
      const start = (currentPage - 1) * itemsPerPage;
      const end = start + itemsPerPage;
      return filteredFields.slice(start, end);
    })()
  );

  // Reset to page 1 when filter changes
  $effect(() => {
    if (searchText) {
      currentPage = 1;
    }
  });

  function applyCondition(text, search, condition) {
    switch (condition) {
      case "contains":
        return text.includes(search);
      case "begins":
        return text.startsWith(search);
      case "ends":
        return text.endsWith(search);
      case "glob":
        // Simple glob: * wildcard
        const pattern = search.replace(/\*/g, ".*");
        return new RegExp(`^${pattern}$`).test(text);
      default:
        return false;
    }
  }

  // Fetch fields với HSCAN
  async function fetchFields(isInitial = true) {
    // Prevent concurrent fetches
    if (isLoading) return;

    if (isInitial) {
      cursor = 0;
      fields = [];
      fieldValues = {};
      hasMore = true;
    }

    if (!hasMore) return;

    isLoading = true;
    const count = cursor === 0 ? 500 : 10000;

    try {
      const result = await invoke("hash_scan", {
        config: $activeConfig,
        db: selectedDb,
        key: selectedKey,
        cursor,
        count,
      });

      cursor = result[0];
      const newFields = result[1];
      fields = [...fields, ...newFields];

      // Load values for preview (limit to first 50 chars)
      for (const field of newFields) {
        try {
          const value = await invoke("hash_get_field", {
            config: $activeConfig,
            db: selectedDb,
            key: selectedKey,
            field,
          });
          fieldValues[field] = value
            ? value.length > 50
              ? value.substring(0, 50) + "..."
              : value
            : "";
        } catch (e) {
          fieldValues[field] = "Error";
        }
      }

      hasMore = cursor !== 0;
    } catch (e) {
      console.error("HSCAN error:", e);
      alert(`Error loading fields: ${e}`);
    } finally {
      isLoading = false;
    }
  }

  // Load field value for editing
  async function loadFieldValue(field) {
    selectedField = field;
    editingFieldName = field; // Set editable field name
    originalFieldName = field; // Track original
    try {
      const value = await invoke("hash_get_field", {
        config: $activeConfig,
        db: selectedDb,
        key: selectedKey,
        field,
      });
      fieldValue = value || "";
      originalFieldValue = value || ""; // Track original
    } catch (e) {
      console.error("HGET error:", e);
      fieldValue = "";
      originalFieldValue = "";
    }
  }

  // Save field value (and rename if field name changed)
  async function saveFieldValue() {
    if (!selectedField) return;

    isSaving = true;
    try {
      // If field name changed, use rename logic
      if (editingFieldName !== selectedField) {
        await invoke("hash_rename_field", {
          config: $activeConfig,
          db: selectedDb,
          key: selectedKey,
          oldField: selectedField,
          newField: editingFieldName,
        });

        // Update fields list
        fields = fields.map((f) =>
          f === selectedField ? editingFieldName : f
        );
        fieldValues[editingFieldName] = fieldValues[selectedField];
        delete fieldValues[selectedField];
        selectedField = editingFieldName;
      }

      // Save the value
      await invoke("hash_set_field", {
        config: $activeConfig,
        db: selectedDb,
        key: selectedKey,
        field: editingFieldName,
        value: fieldValue,
      });

      // Update preview
      fieldValues[editingFieldName] =
        fieldValue.length > 50
          ? fieldValue.substring(0, 50) + "..."
          : fieldValue;

      // Reset originals after successful save
      originalFieldName = editingFieldName;
      originalFieldValue = fieldValue;

      dispatch("refresh");
    } catch (e) {
      console.error("Save error:", e);
      alert(`Error saving field: ${e}`);
    } finally {
      isSaving = false;
    }
  }

  // Add field
  async function addField() {
    const newFieldName = prompt("Enter new field name:");
    if (!newFieldName?.trim()) return;

    try {
      const created = await invoke("hash_add_field", {
        config: $activeConfig,
        db: selectedDb,
        key: selectedKey,
        field: newFieldName.trim(),
      });

      if (!created) {
        alert("Field already exists!");
        return;
      }

      fields = [newFieldName.trim(), ...fields];
      await loadFieldValue(newFieldName.trim());
      dispatch("refresh");
    } catch (e) {
      console.error("HSETNX error:", e);
      alert(`Error adding field: ${e}`);
    }
  }

  // Rename field
  async function renameField(oldName) {
    const newName = prompt("Rename field to:", oldName);
    if (!newName?.trim() || newName === oldName) return;

    try {
      await invoke("hash_rename_field", {
        config: $activeConfig,
        db: selectedDb,
        key: selectedKey,
        oldField: oldName,
        newField: newName.trim(),
      });

      fields = fields.map((f) => (f === oldName ? newName.trim() : f));
      if (selectedField === oldName) {
        selectedField = newName.trim();
      }
      dispatch("refresh");
    } catch (e) {
      console.error("Rename error:", e);
      alert(`Error renaming field: ${e}`);
    }
  }

  // Delete field
  async function deleteField(field) {
    if (!confirm(`Delete field "${field}"?`)) return;

    try {
      await invoke("hash_delete_field", {
        config: $activeConfig,
        db: selectedDb,
        key: selectedKey,
        field,
      });

      fields = fields.filter((f) => f !== field);
      if (selectedField === field) {
        selectedField = "";
        fieldValue = "";
      }
      dispatch("refresh");
    } catch (e) {
      console.error("HDEL error:", e);
      alert(`Error deleting field: ${e}`);
    }
  }

  // Keyboard handler
  function handleKeyDown(e, field) {
    if (e.key === "Backspace") {
      e.preventDefault();
      deleteField(field);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      const idx = fields.indexOf(field);
      if (idx < fields.length - 1) {
        loadFieldValue(fields[idx + 1]);
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const idx = fields.indexOf(field);
      if (idx > 0) {
        loadFieldValue(fields[idx - 1]);
      }
    }
  }

  // Track previous key to avoid infinite loop
  let previousKey = $state("");

  // Init - only run when selectedKey actually changes
  $effect(() => {
    if (selectedKey && selectedKey !== previousKey) {
      previousKey = selectedKey;
      selectedField = "";
      fieldValue = "";
      fetchFields(true);
    }
  });
</script>

<div class="hash-editor">
  <!-- Fields list -->
  <div class="fields-panel">
    {#if showFilter}
      <div class="filter-header">
        <div class="filter-group">
          <select bind:value={searchTarget}>
            <option value="field">Field Name</option>
            <option value="value">Field Value</option>
            <option value="both">Both</option>
          </select>
        </div>
        <div class="filter-group">
          <select bind:value={searchCondition}>
            <option value="contains">Contains</option>
            <option value="begins">Begins With</option>
            <option value="ends">Ends With</option>
            <option value="glob">Matched Glob</option>
          </select>
        </div>
        <div class="filter-group">
          <input
            type="text"
            bind:value={searchText}
            placeholder="Enter search..."
          />
        </div>
      </div>
    {/if}
    <div class="fields-list">
      <table class="fields-table">
        <thead>
          <tr>
            <th>Field</th>
            <th>Value</th>
          </tr>
        </thead>
        <tbody>
          {#each paginatedFields as field}
            <tr
              class="field-row"
              class:selected={field === selectedField}
              onclick={() => loadFieldValue(field)}
              ondblclick={() => renameField(field)}
              tabindex="0"
              role="button"
            >
              <td class="field-name">{field}</td>
              <td class="field-value-preview">{fieldValues[field] || "..."}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="fields-footer">
      <div class="footer-left">
        <button class="btn-add" onclick={addField} title="Add field">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        </button>
        <button
          class="btn-filter"
          onclick={() => (showFilter = !showFilter)}
          title="Toggle filter"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
          </svg>
        </button>
      </div>
      <div class="footer-right">
        {#if totalPages > 1}
          <div class="pagination">
            <button
              class="btn-page"
              onclick={() => (currentPage = Math.max(1, currentPage - 1))}
              disabled={currentPage === 1}
              title="Previous page"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <polyline points="15 18 9 12 15 6"></polyline>
              </svg>
            </button>
            <span class="page-info">{currentPage} of {totalPages}</span>
            <button
              class="btn-page"
              onclick={() =>
                (currentPage = Math.min(totalPages, currentPage + 1))}
              disabled={currentPage === totalPages}
              title="Next page"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <polyline points="9 18 15 12 9 6"></polyline>
              </svg>
            </button>
          </div>
        {/if}
        <span class="fields-count"
          >{filteredFields.length} field{fields.length !== 1 ? "s" : ""}</span
        >
      </div>
    </div>
  </div>

  <!-- Value editor -->
  <div class="value-panel">
    {#if selectedField}
      <div class="dual-inputs">
        <div class="input-group">
          <label>Field Name</label>
          <input
            type="text"
            bind:value={editingFieldName}
            class="field-name-input"
          />
        </div>
        <div class="input-group">
          <label>Field Value</label>
          <div
            class="field-value-input"
            contenteditable="true"
            bind:textContent={fieldValue}
            role="textbox"
            tabindex="0"
          ></div>
        </div>
      </div>
      <div class="editor-footer">
        <button
          class="btn-save"
          onclick={saveFieldValue}
          disabled={!hasChanges || isSaving}
        >
          {isSaving ? "Saving..." : "Save Changes"}
        </button>
      </div>
    {:else}
      <div class="placeholder">Select a field to edit</div>
    {/if}
  </div>
</div>

<style>
  .hash-editor {
    display: flex;
    height: 100%;
    gap: 1px;
    background: #333;
  }

  .fields-panel {
    flex: 0 0 350px;
    display: flex;
    flex-direction: column;
    background: #1a1a1a;
    border-right: 1px solid #333;
  }

  .filter-header {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: #232323;
    border-bottom: 1px solid #333;
    flex-shrink: 0;
  }

  .filter-group {
    display: flex;
  }

  .filter-group label {
    font-size: 0.65rem;
    color: #888;
    font-weight: 600;
    text-transform: uppercase;
  }

  .filter-group select,
  .filter-group input {
    flex: 1;
    padding: 0.3rem 0.5rem;
    background: #1a1a1a;
    color: #e0e0e0;
    border: 1px solid #333;
    border-radius: 3px;
    font-size: 0.75rem;
    font-family: "Monaco", "Menlo", monospace;
  }

  .filter-group select:focus,
  .filter-group input:focus {
    outline: none;
    border-color: #4a9eff;
  }

  .fields-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  /* Custom webkit scrollbar */
  .fields-list::-webkit-scrollbar {
    width: 10px;
  }

  .fields-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .fields-list::-webkit-scrollbar-thumb {
    background: #111;
    border-radius: 3px;
  }

  .fields-list::-webkit-scrollbar-thumb:hover {
    background: #000;
  }

  .fields-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.1rem 1rem;
    background: #232323;
    border-top: 1px solid #333;
    flex-shrink: 0;
  }

  .footer-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.1rem 0;
  }

  .footer-right {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .pagination {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .btn-page {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    color: #4a9eff;
    border: 1px solid #4a9eff;
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.2s;
    padding: 1px 3px;
  }

  .btn-page:hover:not(:disabled) {
    background: #4a9eff;
    color: white;
  }

  .btn-page:disabled {
    opacity: 0.3;
    cursor: not-allowed;
    color: #666;
    border-color: #666;
  }

  .page-info {
    font-size: 0.7rem;
    color: #aaa;
    min-width: 60px;
    text-align: center;
  }

  .fields-count {
    font-size: 0.75rem;
    color: #aaa;
    font-weight: 500;
  }

  .btn-add,
  .btn-filter {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    color: #4a9eff;
    border: 1px solid #4a9eff;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
    padding: 1px 3px;
  }

  .btn-add:hover,
  .btn-filter:hover {
    background: #4a9eff;
    color: white;
  }

  .fields-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }

  .fields-table thead {
    position: sticky;
    top: 0;
    background: #232323;
    z-index: 1;
  }

  .fields-table th {
    padding: 0.5rem 1rem;
    text-align: left;
    font-size: 0.7rem;
    color: #888;
    font-weight: 600;
    text-transform: uppercase;
    border-bottom: 1px solid #333;
    width: 50%;
  }

  .field-row {
    cursor: pointer;
    transition: all 0.15s;
    user-select: none;
  }

  .field-row:hover {
    background: #2d2d2d;
  }

  .field-row.selected {
    background: #4a9eff;
    color: white;
  }

  .field-row td {
    padding: 0.15rem 1rem;
    border-bottom: 1px solid #2a2a2a;
    font-size: 0.75rem;
    font-family: "Monaco", "Menlo", monospace;
    width: 50%;
    max-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .field-name {
    word-break: break-all;
  }

  .field-value-preview {
    color: #999;
  }

  .field-row.selected .field-value-preview {
    color: rgba(255, 255, 255, 0.8);
  }

  .btn-load-more {
    width: 100%;
    padding: 0.75rem;
    background: #2d2d2d;
    color: #4a9eff;
    border: none;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.2s;
  }

  .btn-load-more:hover:not(:disabled) {
    background: #333;
  }

  .btn-load-more:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .value-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #1a1a1a;
  }

  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    background: #232323;
    border-bottom: 1px solid #333;
    flex-shrink: 0;
  }

  .field-label {
    font-size: 0.85rem;
    color: #aaa;
    font-weight: 600;
  }

  .dual-inputs {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1rem;
    gap: 1rem;
    overflow-y: auto;
    min-height: 0;
  }

  /* Custom webkit scrollbar */
  .dual-inputs::-webkit-scrollbar {
    width: 10px;
  }

  .dual-inputs::-webkit-scrollbar-track {
    background: transparent;
  }

  .dual-inputs::-webkit-scrollbar-thumb {
    background: #111;
    border-radius: 3px;
  }

  .dual-inputs::-webkit-scrollbar-thumb:hover {
    background: #000;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .input-group:last-of-type {
    flex: 1;
    min-height: 0;
  }

  .input-group label {
    font-size: 0.75rem;
    color: #888;
    font-weight: 600;
    text-transform: uppercase;
  }

  .field-name-input {
    padding: 0.5rem;
    background: #2a2a2a;
    color: #e0e0e0;
    border: 1px solid #333;
    border-radius: 4px;
    font-family: "Monaco", "Menlo", monospace;
    font-size: 0.85rem;
  }

  .field-name-input:focus {
    outline: none;
    border-color: #4a9eff;
  }

  .field-value-input {
    flex: 1;
    min-height: 0;
    padding: 0.75rem;
    background: #2a2a2a;
    color: #e0e0e0;
    border: 1px solid #333;
    border-radius: 4px;
    font-family: "Monaco", "Menlo", monospace;
    font-size: 13px;
    line-height: 1.6;
    overflow-y: auto;
    overflow-x: hidden;
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  /* Custom scrollbar for contenteditable */
  .field-value-input::-webkit-scrollbar {
    width: 10px;
  }

  .field-value-input::-webkit-scrollbar-track {
    background: transparent;
  }

  .field-value-input::-webkit-scrollbar-thumb {
    background: #111;
    border-radius: 3px;
  }

  .field-value-input::-webkit-scrollbar-thumb:hover {
    background: #000;
  }

  .field-value-input:focus {
    outline: none;
    border-color: #4a9eff;
  }

  .field-value-input:empty:before {
    content: attr(placeholder);
    color: #666;
  }

  .editor-footer {
    padding: 0.2rem 1rem;
    background: #232323;
    border-top: 1px solid #333;
    flex-shrink: 0;
    display: flex;
    justify-content: flex-end;
  }

  .btn-save {
    padding: 0.2rem 1rem;
    background: #4a9eff;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 400;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-save:hover:not(:disabled) {
    background: #3a8eef;
  }

  .btn-save:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    background: #666;
  }

  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #666;
    font-size: 0.9rem;
  }
</style>
