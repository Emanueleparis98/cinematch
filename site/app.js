(() => {
  "use strict";

  const REPO = "Emanueleparis98/cinematch";
  const RELEASE_API = `https://api.github.com/repos/${REPO}/releases/latest`;
  const CACHE_KEY = "cinematch:latest-release:v1";
  const CACHE_MS = 10 * 60 * 1000;

  const setText = (selector, text) => {
    document.querySelectorAll(selector).forEach((node) => {
      node.textContent = text;
    });
  };

  const formatSize = (bytes) => {
    if (!Number.isFinite(bytes) || bytes <= 0) return "installer completo";
    return `${(bytes / (1024 * 1024)).toFixed(1).replace(".", ",")} MB`;
  };

  const safeDownload = (url) =>
    typeof url === "string" &&
    url.startsWith(`https://github.com/${REPO}/releases/download/`);

  const apply = (release) => {
    if (!release || typeof release !== "object") return;
    const assets = Array.isArray(release.assets) ? release.assets : [];
    const setup = assets.find(
      (asset) =>
        typeof asset.name === "string" &&
        /^CineMatch-Setup-.*\.exe$/i.test(asset.name) &&
        safeDownload(asset.browser_download_url)
    );
    const version = String(release.tag_name || "").replace(/^v/i, "");

    if (version) setText("[data-version]", `v${version}`);
    if (setup) {
      document.querySelectorAll("[data-download]").forEach((link) => {
        link.href = setup.browser_download_url;
      });
      setText("[data-size]", formatSize(setup.size));
    }
  };

  const readCache = () => {
    try {
      const raw = sessionStorage.getItem(CACHE_KEY);
      if (!raw) return null;
      const cached = JSON.parse(raw);
      if (!cached || Date.now() - cached.at > CACHE_MS) return null;
      return cached.release;
    } catch {
      return null;
    }
  };

  const writeCache = (release) => {
    try {
      sessionStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), release }));
    } catch {
      // La pagina funziona anche senza cache.
    }
  };

  const load = async () => {
    const cached = readCache();
    if (cached) {
      apply(cached);
      return;
    }
    try {
      const response = await fetch(RELEASE_API, {
        headers: { Accept: "application/vnd.github+json" },
      });
      if (!response.ok) return;
      const release = await response.json();
      apply(release);
      writeCache(release);
    } catch {
      // Fallback: i pulsanti puntano gia' a /releases/latest.
    }
  };

  load();
})();
