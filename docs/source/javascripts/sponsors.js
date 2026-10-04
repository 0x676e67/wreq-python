(() => {
  const initialized = new WeakSet();

  function initialize() {
    document.querySelectorAll("[data-sponsors]").forEach((section) => {
      if (initialized.has(section)) return;
      initialized.add(section);
      const viewport = section.querySelector("[data-sponsor-window]");
      const originals = [...viewport.children];
      const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
      if (!originals.length || reducedMotion.matches) return;

      // Repeat the row for seamless wrapping; only the original links are tab stops.
      for (let repeat = 0; repeat < 2; repeat++) {
        originals.forEach((card) => {
          const copy = card.cloneNode(true);
          copy.setAttribute("aria-hidden", "true");
          copy.tabIndex = -1;
          viewport.append(copy);
        });
      }
      const nextRow = viewport.children[originals.length];
      const speed = 40; // Pixels per second.
      let previous;
      let position = viewport.scrollLeft;
      function animate(time) {
        if (!section.isConnected) return;
        const elapsed = previous === undefined ? 0 : Math.min(time - previous, 64);
        previous = time;
        if (!reducedMotion.matches && !document.hidden
          && !viewport.matches(":hover") && !section.contains(document.activeElement)) {
          const width = nextRow.offsetLeft - originals[0].offsetLeft;
          if (width > 0) {
            position = (position + elapsed * speed / 1000) % width;
            viewport.scrollLeft = position;
          }
        } else {
          position = viewport.scrollLeft;
        }
        window.requestAnimationFrame(animate);
      }
      window.requestAnimationFrame(animate);
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initialize, { once: true });
  } else {
    initialize();
  }
  if (typeof document$ !== "undefined") document$.subscribe(initialize);
})();
