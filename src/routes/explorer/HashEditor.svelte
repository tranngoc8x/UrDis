<script>
  import { invoke } from "@tauri-apps/api/core";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { activeConfig } from "$lib/stores.js";
  import { createEventDispatcher } from "svelte";

  let { selectedKey = "", selectedDb = 0 } = $props();

  const dispatch = createEventDispatcher();

  // State
  let fields = $state([]);
  let fieldValues = $state({}); // Cache field values for table display
  let selectedFields = $state([]); // Array of selected fields
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

  // Context menu states
  let showContextMenu = $state(false);
  let contextMenuPosition = $state({ x: 0, y: 0 });
  let contextMenuField = $state("");

  // Confirm dialog states
  let showConfirmDialog = $state(false);
  let confirmDialogMessage = $state("");
  let confirmDialogCallback = $state(() => {});

  // TTL Dialog states
  let showTTLDialog = $state(false);
  let ttlValue = $state(0);
  let ttlUnit = $state("seconds"); // seconds, minutes, hours, days, weeks, months, years
  let ttlTargetFields = $state([]);

  // Redis version check for HEXPIRE support (>= 7.4)
  let redisVersion = $state("");
  let supportsFieldTTL = $derived(
    (() => {
      const parts = redisVersion.split(".");
      if (parts.length < 2) return false;
      const major = parseInt(parts[0], 10);
      const minor = parseInt(parts[1], 10);
      return major > 7 || (major === 7 && minor >= 4);
    })()
  );

  // Keyboard navigation
  let selectedIndex = $state(0);

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
  async function loadFieldValue(field, event = null) {
    console.log(
      `[HashEditor] loadFieldValue: ${field}, button: ${event?.button}`
    );

    // Handle Right Click (button 2)
    if (event && event.button === 2) {
      const alreadySelected = selectedFields.some((f) => f === field);
      if (alreadySelected) {
        console.log(
          `[HashEditor] Right-click on SELECTED field: ${field}. Keeping selection.`
        );
        // Preservation: if already selected, do nothing to selection
      } else {
        console.log(
          `[HashEditor] Right-click on UNSELECTED field: ${field}. Selecting single.`
        );
        selectedFields = [field];
      }
    } else {
      // Handle Multi-select for Fields (Left Click)
      if (event && (event.ctrlKey || event.metaKey)) {
        if (selectedFields.includes(field)) {
          selectedFields = selectedFields.filter((f) => f !== field);
        } else {
          selectedFields = [...selectedFields, field];
        }
      } else if (event && event.shiftKey && selectedFields.length > 0) {
        const lastField = selectedFields[selectedFields.length - 1];
        const startIdx = paginatedFields.indexOf(lastField);
        const endIdx = paginatedFields.indexOf(field);
        if (startIdx !== -1 && endIdx !== -1) {
          const [min, max] =
            startIdx < endIdx ? [startIdx, endIdx] : [endIdx, startIdx];
          const range = paginatedFields.slice(min, max + 1);
          selectedFields = range;
        } else {
          selectedFields = [field];
        }
      } else {
        selectedFields = [field];
      }
    }

    // Always attempt to load value if exactly one field is in selection
    if (selectedFields.length === 1) {
      const targetField = selectedFields[0];
      editingFieldName = targetField;
      originalFieldName = targetField;
      try {
        const value = await invoke("hash_get_field", {
          config: $activeConfig,
          db: selectedDb,
          key: selectedKey,
          field: targetField,
        });
        fieldValue = value || "";
        originalFieldValue = value || "";
      } catch (e) {
        console.error("HGET error:", e);
        fieldValue = "";
        originalFieldValue = "";
      }
    } else {
      editingFieldName = "";
      fieldValue = "";
      originalFieldName = "";
      originalFieldValue = "";
    }
  }

  // Save field value (and rename if field name changed)
  async function saveFieldValue() {
    const currentField = selectedFields[0];
    if (!currentField) return;

    isSaving = true;
    try {
      // If field name changed, use rename logic
      if (editingFieldName !== currentField) {
        await invoke("hash_rename_field", {
          config: $activeConfig,
          db: selectedDb,
          key: selectedKey,
          oldField: currentField,
          newField: editingFieldName,
        });

        // Update fields list
        fields = fields.map((f) => (f === currentField ? editingFieldName : f));
        fieldValues[editingFieldName] = fieldValues[currentField];
        delete fieldValues[currentField];
        selectedFields = [editingFieldName];
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

      if (selectedFields[0] !== editingFieldName) {
        selectedFields = [editingFieldName];
      }

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
      selectedFields = [newFieldName.trim()];
      dispatch("refresh");
    } catch (e) {
      console.error("HSETNX error:", e);
      alert(`Error adding field: ${e}`);
    }
  }

  // Rename field
  async function renameField(oldName) {
    console.log("Rename field called for:", oldName);
    const newName = prompt("Rename field to:", oldName);
    if (!newName?.trim() || newName === oldName) return;

    try {
      console.log("Renaming", oldName, "to", newName);
      await invoke("hash_rename_field", {
        config: $activeConfig,
        db: selectedDb,
        key: selectedKey,
        oldField: oldName,
        newField: newName.trim(),
      });

      fields = fields.map((f) => (f === oldName ? newName.trim() : f));
      if (selectedFields.includes(oldName)) {
        selectedFields = selectedFields.map((f) =>
          f === oldName ? newName.trim() : f
        );
      }
      dispatch("refresh");
      console.log("✓ Field renamed successfully");
    } catch (e) {
      console.error("Rename error:", e);
      alert(`Error renaming field: ${e}`);
    }
  }

  // Delete field(s)
  async function deleteField(field = null) {
    const fieldsToDelete = field ? [field] : selectedFields;
    if (fieldsToDelete.length === 0) return;

    const message =
      fieldsToDelete.length === 1
        ? `Delete field "${fieldsToDelete[0]}"?`
        : `Delete ${fieldsToDelete.length} fields?`;

    showConfirm(message, async () => {
      console.log("Deleting fields:", fieldsToDelete);
      try {
        for (const f of fieldsToDelete) {
          await invoke("hash_delete_field", {
            config: $activeConfig,
            db: selectedDb,
            key: selectedKey,
            field: f,
          });
        }

        fields = fields.filter((f) => !fieldsToDelete.includes(f));
        selectedFields = selectedFields.filter(
          (f) => !fieldsToDelete.includes(f)
        );

        if (selectedFields.length === 0) {
          editingFieldName = "";
          fieldValue = "";
        } else if (selectedFields.length === 1) {
          await loadFieldValue(selectedFields[0]);
        }

        dispatch("refresh");
        console.log("✓ Fields deleted successfully");
      } catch (e) {
        console.error("HDEL error:", e);
        alert(`Error deleting fields: ${e}`);
      }
    });
  }

  // Keyboard handler
  function handleKeyDown(e, field) {
    if (e.key === "Backspace" || e.key === "Delete") {
      e.preventDefault();
      deleteField();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      const lastSelected =
        selectedFields[selectedFields.length - 1] || fields[0];
      const idx = fields.indexOf(lastSelected);
      if (idx < fields.length - 1) {
        loadFieldValue(fields[idx + 1]);
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const firstSelected = selectedFields[0] || fields[0];
      const idx = fields.indexOf(firstSelected);
      if (idx > 0) {
        loadFieldValue(fields[idx - 1]);
      }
    }
  }

  // Context Menu Actions
  function showFieldContextMenu(event, field) {
    event.preventDefault();
    const alreadySelected = selectedFields.some((f) => f === field);
    if (!alreadySelected) {
      selectedFields = [field];
    }
    contextMenuField = field;
    contextMenuPosition = { x: event.clientX, y: event.clientY };
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
    contextMenuField = "";
  }

  // Confirm dialog helpers
  function showConfirm(message, callback) {
    console.log("ShowConfirm called with:", message);
    confirmDialogMessage = message;
    confirmDialogCallback = callback;
    showConfirmDialog = true;
  }

  function handleConfirmYes() {
    console.log("Confirm YES clicked");
    showConfirmDialog = false;
    if (confirmDialogCallback) {
      console.log("Executing callback...");
      confirmDialogCallback();
    }
  }

  function handleConfirmNo() {
    console.log("Confirm NO clicked");
    showConfirmDialog = false;
    confirmDialogCallback = () => {};
  }

  // Clipboard operations
  async function copyToClipboard(text) {
    try {
      await navigator.clipboard.writeText(text);
      console.log("✓ Copied to clipboard:", text.substring(0, 100));
    } catch (err) {
      console.error("Failed to copy:", err);
      alert("Failed to copy to clipboard");
    }
  }

  async function copyFieldNames(single = false) {
    let names = "";
    if (single && contextMenuField) {
      names = contextMenuField;
    } else {
      names = Array.from(selectedFields).join("\n");
    }

    if (names) {
      console.log(
        `[HashEditor] Copying ${single ? "single" : "selected list (" + selectedFields.length + ")"}`
      );
      await copyToClipboard(names);
    }
    closeContextMenu();
  }

  async function copyFieldValue() {
    const field = contextMenuField || selectedFields[0];
    if (field && fieldValues[field]) {
      console.log("Copying field value for:", field);
      await copyToClipboard(fieldValues[field]);
      closeContextMenu();
    }
  }

  async function copyAsJSON() {
    const field = contextMenuField || selectedFields[0];
    if (field && fieldValues[field]) {
      const json = JSON.stringify({ [field]: fieldValues[field] }, null, 2);
      console.log("Copying as JSON:", json.substring(0, 200));
      await copyToClipboard(json);
      closeContextMenu();
    }
  }

  async function copyAsHSET() {
    const field = contextMenuField || selectedFields[0];
    if (field && fieldValues[field]) {
      const command = `HSET ${selectedKey} "${field}" "${fieldValues[field]}"`;
      console.log("Copying as HSET command:", command.substring(0, 200));
      await copyToClipboard(command);
      closeContextMenu();
    }
  }

  async function copyAllAsJSON() {
    if (selectedFields.length === 0) {
      closeContextMenu();
      return;
    }

    const jsonObject = {};
    for (const field of selectedFields) {
      if (fieldValues[field] !== undefined) {
        jsonObject[field] = fieldValues[field];
      }
    }

    const json = JSON.stringify(jsonObject, null, 2);
    console.log(
      `Copying ${selectedFields.length} fields as JSON:`,
      json.substring(0, 200)
    );
    await copyToClipboard(json);
    closeContextMenu();
  }

  async function copyAllFieldValues() {
    if (selectedFields.length === 0) {
      closeContextMenu();
      return;
    }

    const values = selectedFields
      .map((field) => fieldValues[field])
      .filter((v) => v !== undefined)
      .join("\n");

    console.log(`Copying ${selectedFields.length} field values`);
    await copyToClipboard(values);
    closeContextMenu();
  }

  // Check Redis version when component mounts
  async function checkRedisVersion() {
    try {
      const config = $activeConfig;
      if (!config) return;

      const version = await invoke("get_redis_version", { config });
      redisVersion = version;
      console.log(
        "[HashEditor] Redis version:",
        version,
        "Supports field TTL:",
        supportsFieldTTL
      );
    } catch (e) {
      console.error("Failed to get Redis version:", e);
      redisVersion = "";
    }
  }

  // TTL Dialog functions
  function openFieldTTLDialog(fields) {
    ttlTargetFields = Array.isArray(fields) ? fields : [fields];
    ttlValue = 0;
    ttlUnit = "seconds";
    showTTLDialog = true;
    closeContextMenu();
  }

  function closeFieldTTLDialog() {
    showTTLDialog = false;
    ttlTargetFields = [];
  }

  async function saveFieldTTL() {
    if (ttlTargetFields.length === 0 || ttlValue <= 0) {
      closeFieldTTLDialog();
      return;
    }

    // Convert to seconds
    let seconds = ttlValue;
    switch (ttlUnit) {
      case "minutes":
        seconds = ttlValue * 60;
        break;
      case "hours":
        seconds = ttlValue * 3600;
        break;
      case "days":
        seconds = ttlValue * 86400;
        break;
      case "weeks":
        seconds = ttlValue * 604800;
        break;
      case "months":
        seconds = ttlValue * 2592000;
        break; // 30 days
      case "years":
        seconds = ttlValue * 31536000;
        break; // 365 days
    }

    try {
      const config = $activeConfig;
      if (!config) return;

      await invoke("hash_set_field_ttl", {
        config,
        db: selectedDb,
        key: selectedKey,
        fields: ttlTargetFields,
        ttl: seconds,
      });

      console.log(
        `[HashEditor] Set TTL for ${ttlTargetFields.length} field(s) to ${seconds} seconds`
      );
    } catch (error) {
      console.error("Failed to set field TTL:", error);
      alert("Failed to set field TTL: " + error);
    } finally {
      closeFieldTTLDialog();
    }
  }

  // Track previous key to avoid infinite loop
  let previousKey = $state("");

  // Init - only run when selectedKey actually changes
  $effect(() => {
    if (selectedKey && selectedKey !== previousKey) {
      previousKey = selectedKey;
      selectedFields = [];
      editingFieldName = "";
      fieldValue = "";
      fetchFields(true);
      checkRedisVersion();
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

    <!-- Confirm Dialog -->
    {#if showConfirmDialog}
      <div class="confirm-dialog-overlay" onclick={handleConfirmNo}>
        <div class="confirm-dialog" onclick={(e) => e.stopPropagation()}>
          <div class="confirm-dialog-message">{confirmDialogMessage}</div>
          <div class="confirm-dialog-buttons">
            <button class="btn-confirm-no" onclick={handleConfirmNo}
              >Cancel</button
            >
            <button class="btn-confirm-yes" onclick={handleConfirmYes}
              >Delete</button
            >
          </div>
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
        <tbody tabindex="-1">
          {#each paginatedFields as field}
            <tr
              class="field-row"
              class:selected={selectedFields.includes(field)}
              onmousedown={(e) => {
                if (e.button === 1) return; // Ignore middle click
                console.log(
                  "[HashEditor] TR Mousedown:",
                  field,
                  "button:",
                  e.button
                );
                e.stopPropagation();
                loadFieldValue(field, e);
              }}
              oncontextmenu={(e) => showFieldContextMenu(e, field)}
              onkeydown={(e) => handleKeyDown(e, field)}
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
          >{filteredFields.length} field{fields.length !== 1
            ? "s"
            : ""}{selectedFields.length > 1
            ? ` (${selectedFields.length} selected)`
            : ""}</span
        >
      </div>
    </div>
  </div>

  <!-- Value editor -->
  <div class="value-panel">
    {#if selectedFields.length === 1}
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
    {:else if selectedFields.length > 1}
      <div class="placeholder">
        {selectedFields.length} fields selected. Right-click for actions.
      </div>
    {:else}
      <div class="placeholder">Select a field to edit</div>
    {/if}
  </div>
</div>

<!-- TTL Dialog -->
{#if showTTLDialog}
  <div class="ttl-dialog-overlay" onclick={closeFieldTTLDialog}>
    <div class="ttl-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="ttl-dialog-header">
        <h3>Set TTL for Field(s)</h3>
        <span class="ttl-dialog-key">
          {ttlTargetFields.length === 1
            ? ttlTargetFields[0]
            : `${ttlTargetFields.length} fields`}
        </span>
      </div>
      <div class="ttl-dialog-body">
        <div class="ttl-input-group">
          <input
            type="number"
            min="1"
            bind:value={ttlValue}
            placeholder="Enter value"
            class="ttl-input"
          />
          <select bind:value={ttlUnit} class="ttl-select">
            <option value="seconds">Seconds</option>
            <option value="minutes">Minutes</option>
            <option value="hours">Hours</option>
            <option value="days">Days</option>
            <option value="weeks">Weeks</option>
            <option value="months">Months</option>
            <option value="years">Years</option>
          </select>
        </div>
      </div>
      <div class="ttl-dialog-footer">
        <button class="btn-ttl-cancel" onclick={closeFieldTTLDialog}
          >Cancel</button
        >
        <button class="btn-ttl-save" onclick={saveFieldTTL}>Save</button>
      </div>
    </div>
  </div>
{/if}

<!-- Context Menu: Single Field -->
{#if showContextMenu && selectedFields.length === 1}
  <div
    class="context-menu-overlay"
    onclick={closeContextMenu}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <div
      class="context-menu"
      style="left: {contextMenuPosition.x}px; top: {contextMenuPosition.y}px;"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="context-menu-item" onclick={() => copyFieldNames(true)}>
        <i class="codicon codicon-copy"></i>
        <span>Copy Field Name</span>
      </div>
      <div class="context-menu-item" onclick={copyFieldValue}>
        <i class="codicon codicon-copy"></i>
        <span>Copy Field Value</span>
      </div>
      <div class="context-menu-item" onclick={copyAsJSON}>
        <i class="codicon codicon-json"></i>
        <span>Copy as JSON</span>
      </div>
      {#if supportsFieldTTL}
        <div
          class="context-menu-item"
          onclick={() => openFieldTTLDialog(contextMenuField)}
        >
          <i class="codicon codicon-clock"></i>
          <span>Set TTL</span>
        </div>
      {/if}

      <div class="context-menu-separator"></div>
      <div
        class="context-menu-item danger"
        onclick={() => deleteField(contextMenuField)}
      >
        <i class="codicon codicon-trash"></i>
        <span>Delete Field</span>
      </div>
    </div>
  </div>
{/if}

<!-- Context Menu: Multiple Fields -->
{#if showContextMenu && selectedFields.length > 1}
  <div
    class="context-menu-overlay"
    onclick={closeContextMenu}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <div
      class="context-menu"
      style="left: {contextMenuPosition.x}px; top: {contextMenuPosition.y}px;"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="context-menu-item" onclick={() => copyFieldNames(false)}>
        <i class="codicon codicon-copy"></i>
        <span>Copy All Names</span>
      </div>
      <div class="context-menu-item" onclick={copyAllFieldValues}>
        <i class="codicon codicon-copy"></i>
        <span>Copy All Values</span>
      </div>
      <div class="context-menu-item" onclick={copyAllAsJSON}>
        <i class="codicon codicon-json"></i>
        <span>Copy All as JSON</span>
      </div>

      {#if supportsFieldTTL}
        <div
          class="context-menu-item"
          onclick={() => openFieldTTLDialog(selectedFields)}
        >
          <i class="codicon codicon-clock"></i>
          <span>Set TTL for All</span>
        </div>
      {/if}

      <div class="context-menu-separator"></div>
      <div class="context-menu-item danger" onclick={() => deleteField()}>
        <i class="codicon codicon-trash"></i>
        <span>Delete All Fields</span>
      </div>
    </div>
  </div>
{/if}

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

  /* Context Menu */
  .context-menu-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1000;
  }

  .context-menu {
    position: fixed;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    min-width: 200px;
    padding: 0.25rem 0;
    z-index: 1001;
  }

  .context-menu-item {
    padding: 0.5rem 1rem;
    cursor: pointer;
    font-size: 0.85rem;
    color: #e0e0e0;
    transition: all 0.15s;
  }

  .context-menu-item:hover {
    background: #3a3a3a;
  }

  .context-menu-item.danger {
    color: #ff6b6b;
  }

  .context-menu-item.danger:hover {
    background: #4a2020;
  }

  .context-menu-separator {
    height: 1px;
    background: #444;
    margin: 0.25rem 0;
  }

  /* Confirm Dialog */
  .confirm-dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }

  .confirm-dialog {
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 8px;
    padding: 1.5rem;
    min-width: 300px;
    max-width: 400px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.7);
  }

  .confirm-dialog-message {
    color: #e0e0e0;
    font-size: 0.95rem;
    margin-bottom: 1.5rem;
    line-height: 1.5;
  }

  .confirm-dialog-buttons {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  .btn-confirm-no,
  .btn-confirm-yes {
    padding: 0.5rem 1.25rem;
    border: none;
    border-radius: 4px;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-confirm-no {
    background: #3a3a3a;
    color: #e0e0e0;
  }

  .btn-confirm-no:hover {
    background: #4a4a4a;
  }

  .btn-confirm-yes {
    background: #d32f2f;
    color: white;
  }

  .btn-confirm-yes:hover {
    background: #c62828;
  }

  /* TTL Dialog Styles */
  .ttl-dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10002;
  }

  .ttl-dialog {
    background: #252526;
    border: 1px solid #454545;
    border-radius: 8px;
    min-width: 320px;
    max-width: 400px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    animation: dialogScale 0.2s ease-out;
  }

  .ttl-dialog-header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #333;

    h3 {
      margin: 0 0 0.25rem 0;
      font-size: 1rem;
      font-weight: 600;
      color: #ffffff;
    }

    .ttl-dialog-key {
      font-size: 0.8rem;
      color: #888;
      font-family: monospace;
      word-break: break-all;
    }
  }

  .ttl-dialog-body {
    padding: 1.25rem;
  }

  .ttl-input-group {
    display: flex;
    gap: 0.5rem;
  }

  .ttl-input {
    flex: 1;
    padding: 0.6rem 0.75rem;
    border: 1px solid #454545;
    border-radius: 4px;
    background: #1e1e1e;
    color: #cccccc;
    font-size: 0.9rem;

    &:focus {
      outline: none;
      border-color: #007acc;
    }
  }

  .ttl-select {
    padding: 0.6rem 0.75rem;
    border: 1px solid #454545;
    border-radius: 4px;
    background: #1e1e1e;
    color: #cccccc;
    font-size: 0.9rem;
    cursor: pointer;

    &:focus {
      outline: none;
      border-color: #007acc;
    }
  }

  .ttl-dialog-footer {
    padding: 0.75rem 1.25rem;
    border-top: 1px solid #333;
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .btn-ttl-cancel,
  .btn-ttl-save {
    padding: 0.5rem 1.25rem;
    border-radius: 4px;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
  }

  .btn-ttl-cancel {
    background: #3c3c3c;
    color: #cccccc;

    &:hover {
      background: #4c4c4c;
    }
  }

  .btn-ttl-save {
    background: #0e639c;
    color: white;

    &:hover {
      background: #1177bb;
    }
  }

  @keyframes dialogScale {
    from {
      opacity: 0;
      transform: scale(0.9);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
