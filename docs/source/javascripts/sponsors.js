(() => {
  const initialized = new WeakSet();

  function initialize() {
    document.querySelectorAll("[data-sponsors]").forEach((section) => {
      if (initialized.has(section)) return;
      initialized.add(section);

      const viewport = section.querySelector("[data-sponsor-window]");
      const controls = section.querySelector(".wreq-carousel-controls");
      const pause = section.querySelector("[data-sponsor-pause]");
      const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
      let paused = false;
      let hovering = false;

      controls.hidden = false;

      function move(direction) {
        const card = viewport.querySelector(".wreq-sponsor");
        const gap = parseFloat(window.getComputedStyle(viewport).columnGap) || 0;
        const step = card.getBoundingClientRect().width + gap;
        const max = viewport.scrollWidth - viewport.clientWidth;
        const current = viewport.scrollLeft;
        const left = direction > 0
          ? (current >= max - 1 ? 0 : Math.min(max, current + step))
          : (current <= 1 ? max : Math.max(0, current - step));
        viewport.scrollTo({
          left,
          behavior: reducedMotion.matches ? "instant" : "smooth",
        });
      }

      pause.addEventListener("click", () => {
        paused = !paused;
        pause.setAttribute("aria-pressed", String(paused));
        pause.textContent = paused ? "Resume" : "Pause";
      });
      section.querySelector("[data-sponsor-previous]").addEventListener("click", () => move(-1));
      section.querySelector("[data-sponsor-next]").addEventListener("click", () => move(1));
      section.addEventListener("mouseenter", () => { hovering = true; });
      section.addEventListener("mouseleave", () => { hovering = false; });

      const timer = window.setInterval(() => {
        if (!section.isConnected) {
          window.clearInterval(timer);
          return;
        }
        if (paused || hovering || reducedMotion.matches || document.hidden || section.contains(document.activeElement)) return;
        if (viewport.scrollWidth > viewport.clientWidth + 1) move(1);
      }, 5000);
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initialize, { once: true });
  } else {
    initialize();
  }
  if (typeof document$ !== "undefined") document$.subscribe(initialize);
})();
