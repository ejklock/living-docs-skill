/**
 * Cmd+K search palette, a progressive enhancement over the plain
 * `GET /?q=` search form (ADR 0015, issue 0008 slice S4). Vanilla JS, no
 * dependencies: Control+K or Meta+K toggles a hidden overlay, typing
 * debounces a fetch to `GET /palette?q=<term>` and injects the returned,
 * server-escaped HTML fragment into the results container. With this
 * script absent, the overlay stays hidden and the plain search form
 * remains the fully functional fallback.
 */
(function paletteEnhancement() {
  "use strict";

  var DEBOUNCE_MS = 150;

  /**
   * @param {Function} fn Function to defer until calls settle.
   * @param {number} waitMs Idle time required before `fn` runs.
   * @returns {Function} Debounced wrapper around `fn`.
   */
  function debounce(fn, waitMs) {
    var timerId = null;
    return function debounced() {
      var callArgs = arguments;
      var callContext = this;
      window.clearTimeout(timerId);
      timerId = window.setTimeout(function runDebounced() {
        fn.apply(callContext, callArgs);
      }, waitMs);
    };
  }

  function isToggleShortcut(event) {
    return (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k";
  }

  function openPalette(overlay, input) {
    overlay.hidden = false;
    input.focus();
  }

  function closePalette(overlay, input, resultsEl) {
    overlay.hidden = true;
    input.value = "";
    resultsEl.innerHTML = "";
  }

  function togglePalette(overlay, input, resultsEl) {
    if (overlay.hidden) {
      openPalette(overlay, input);
    } else {
      closePalette(overlay, input, resultsEl);
    }
  }

  function fetchResultsFragment(query) {
    return window
      .fetch("/palette?q=" + encodeURIComponent(query))
      .then(function readFragmentText(response) {
        return response.text();
      });
  }

  function renderResults(resultsEl, query) {
    if (query.trim() === "") {
      resultsEl.innerHTML = "";
      return;
    }
    fetchResultsFragment(query).then(function injectFragment(html) {
      resultsEl.innerHTML = html;
    });
  }

  function wireKeyboardShortcuts(overlay, input, resultsEl) {
    document.addEventListener("keydown", function handleKeydown(event) {
      if (isToggleShortcut(event)) {
        event.preventDefault();
        togglePalette(overlay, input, resultsEl);
        return;
      }
      if (event.key === "Escape" && !overlay.hidden) {
        closePalette(overlay, input, resultsEl);
      }
    });
  }

  function wirePaletteInput(input, resultsEl) {
    var onInput = debounce(function handleInput() {
      renderResults(resultsEl, input.value);
    }, DEBOUNCE_MS);
    input.addEventListener("input", onInput);
  }

  document.addEventListener("DOMContentLoaded", function initPalette() {
    var overlay = document.getElementById("palette-overlay");
    var input = document.getElementById("palette-input");
    var resultsEl = document.getElementById("palette-results");
    if (!overlay || !input || !resultsEl) {
      return;
    }
    wireKeyboardShortcuts(overlay, input, resultsEl);
    wirePaletteInput(input, resultsEl);
  });
})();
