import SimpleBar from "simplebar";

/**
 * Shared SimpleBar Svelte action.
 * Allows components to easily attach a custom scrollbar to any element.
 *
 * Usage:
 * <div use:simplebar>Content</div>
 *
 * @param {HTMLElement} node
 * @param {Object} options
 */
export function simplebar(node, options = { autoHide: false }) {
  const sb = new SimpleBar(node, options);

  // Auto-recalculate when content changes
  const observer = new MutationObserver(() => sb.recalculate());
  observer.observe(node, {
    childList: true,
    subtree: true,
    characterData: true,
  });

  return {
    destroy() {
      observer.disconnect();
      sb.unMount();
    },
  };
}
