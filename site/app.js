(() => {
  const tabs = [...document.querySelectorAll("[data-tab]")];
  const panels = [...document.querySelectorAll("[data-panel]")];

  const selectTab = (name) => {
    tabs.forEach((tab) => {
      const active = tab.dataset.tab === name;
      tab.classList.toggle("is-active", active);
      tab.setAttribute("aria-selected", String(active));
    });
    panels.forEach((panel) => {
      const active = panel.dataset.panel === name;
      panel.classList.toggle("is-active", active);
      panel.hidden = !active;
    });
  };

  tabs.forEach((tab) => {
    tab.addEventListener("click", () => selectTab(tab.dataset.tab));
    tab.addEventListener("keydown", (event) => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      const index = tabs.indexOf(tab);
      let next = index;
      if (event.key === "ArrowRight") next = (index + 1) % tabs.length;
      if (event.key === "ArrowLeft") next = (index - 1 + tabs.length) % tabs.length;
      if (event.key === "Home") next = 0;
      if (event.key === "End") next = tabs.length - 1;
      tabs[next].focus();
      selectTab(tabs[next].dataset.tab);
    });
  });

  document.querySelectorAll("[data-copy]").forEach((button) => {
    button.addEventListener("click", async () => {
      const target = document.getElementById(button.dataset.copy);
      if (!target) return;
      const text = target.textContent.replace(/\s+/g, " ").trim();
      try {
        await navigator.clipboard.writeText(text);
        const previous = button.textContent;
        button.textContent = "Copiato";
        setTimeout(() => { button.textContent = previous; }, 1600);
      } catch {
        button.textContent = "Copia manuale";
      }
    });
  });

  const setVersion = (version) => {
    document.querySelectorAll("[data-version]").forEach((node) => {
      node.textContent = version;
    });
  };

  const fillVersion = async () => {
    try {
      const response = await fetch("https://api.github.com/repos/Emanueleparis98/termovies/releases/latest", {
        headers: { Accept: "application/vnd.github+json" },
      });
      if (!response.ok) return;
      const release = await response.json();
      const version = String(release.tag_name || "").replace(/^terminal-v/, "");
      if (!/^\d+\.\d+\.\d+$/.test(version)) return;
      setVersion(version);

      const arch = (release.assets || []).find((asset) => /-1-any\.pkg\.tar\.zst$/.test(asset.name));
      if (arch) {
        const link = document.querySelector("[data-arch-download]");
        if (link) link.href = arch.browser_download_url;
        const label = document.querySelector("[data-arch-label]");
        if (label) label.textContent = arch.name;
      }
    } catch {
      // Static links already point to the latest release page.
    }
  };

  document.querySelectorAll("[data-year]").forEach((node) => {
    node.textContent = String(new Date().getFullYear());
  });

  fillVersion();
})();
