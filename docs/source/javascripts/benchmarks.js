(() => {
  const initialized = new WeakSet();

  function initialize() {
    document.querySelectorAll("[data-bench-explorer]").forEach((section) => {
      if (initialized.has(section)) return;
      initialized.add(section);
      const catalog = JSON.parse(section.querySelector("[data-chart-catalog]").textContent);
      const selects = [...section.querySelectorAll("[data-chart-select]")];
      const payloadButtons = [...section.querySelectorAll("[data-chart-payload]")];
      let payload = Number(payloadButtons.find((button) => button.getAttribute("aria-pressed") === "true").dataset.chartPayload);
      const image = section.querySelector("[data-chart-image]");
      const mobile = section.querySelector("[data-chart-mobile]");
      // The docs builder rewrites image URLs for nested pages and subpath hosting.
      const prefix = new URL(".", image.src).href;
      const number = new Intl.NumberFormat("en-US", { minimumFractionDigits: 1, maximumFractionDigits: 1 });

      function update() {
        const values = Object.fromEntries(selects.map((select) => [select.dataset.chartSelect, select.value]));
        const current = catalog.cases.find((item) => item.payload_bytes === payload
          && item.api === values.api && item.protocol === values.protocol
          && item.body_kind === values.body_kind && item.concurrency === Number(values.concurrency));
        if (!current) return;
        const assets = current.assets.dark;
        image.src = `${prefix}${assets.desktop}`;
        const description = [...current.rows.map((row) => `${row.label}: ${number.format(row.rps)}`),
          ...current.unsupported.map((label) => `${label}: N/A`)].join("; ");
        image.alt = `${current.title}. Throughput in requests per second. ${description}.`;
        mobile.srcset = `${prefix}${assets.mobile}`;
        section.querySelector("[data-chart-caption]").textContent = current.title;
        section.querySelector("[data-chart-unsupported]").textContent = current.unsupported.length
          ? `N/A: ${current.unsupported.join(", ")}` : "All shown clients support this case.";
        payloadButtons.forEach((button) => button.setAttribute("aria-pressed", String(Number(button.dataset.chartPayload) === payload)));
      }

      function move(direction) {
        const index = payloadButtons.findIndex((button) => Number(button.dataset.chartPayload) === payload);
        const next = (index + direction + payloadButtons.length) % payloadButtons.length;
        payload = Number(payloadButtons[next].dataset.chartPayload);
        update();
      }
      payloadButtons.forEach((button) => {
        button.addEventListener("click", () => {
          payload = Number(button.dataset.chartPayload);
          update();
        });
        button.addEventListener("keydown", (event) => {
          if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
          event.preventDefault();
          if (event.key === "Home" || event.key === "End") {
            payload = Number(payloadButtons[event.key === "Home" ? 0 : payloadButtons.length - 1].dataset.chartPayload);
            update();
          } else move(event.key === "ArrowRight" ? 1 : -1);
          payloadButtons.find((item) => Number(item.dataset.chartPayload) === payload).focus();
        });
      });
      section.querySelector("[data-chart-previous]").addEventListener("click", () => move(-1));
      section.querySelector("[data-chart-next]").addEventListener("click", () => move(1));
      selects.forEach((select) => select.addEventListener("change", update));
      section.querySelectorAll("[data-chart-controls]").forEach((controls) => { controls.hidden = false; });
      update();
    });
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", initialize, { once: true });
  else initialize();
  if (typeof document$ !== "undefined") document$.subscribe(initialize);
})();
