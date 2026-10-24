// Enlaza los botones de descarga con el instalador de la última versión publicada.
// Si la API de GitHub no responde, los botones siguen apuntando a la página de versiones.
(function () {
  "use strict";

  var REPO = "CodingChar/DisorderLauncher";
  var buttons = [document.getElementById("dl-main"), document.getElementById("dl-second")];
  var metas = [document.getElementById("dl-meta"), document.getElementById("dl-meta2")];
  var detail = document.getElementById("dl-detail");
  var hashNote = document.getElementById("hash-note");

  function fmtSize(bytes) {
    return (bytes / (1024 * 1024)).toFixed(1).replace(".", ",") + " MB";
  }

  function pickInstaller(assets) {
    var exe = assets.filter(function (a) { return /setup.*\.exe$/i.test(a.name); })[0];
    return exe || assets.filter(function (a) { return /\.(exe|msi)$/i.test(a.name); })[0] || null;
  }

  function showHash(asset) {
    // GitHub publica el SHA-256 de cada archivo como "sha256:<hex>".
    if (!asset.digest || asset.digest.indexOf("sha256:") !== 0) return;
    var code = document.createElement("code");
    code.textContent = asset.digest.slice(7);
    hashNote.textContent = "SHA-256 de " + asset.name + ": ";
    hashNote.appendChild(code);
    hashNote.hidden = false;
  }

  fetch("https://api.github.com/repos/" + REPO + "/releases/latest", {
    headers: { Accept: "application/vnd.github+json" }
  })
    .then(function (r) { return r.ok ? r.json() : Promise.reject(new Error("sin versiones")); })
    .then(function (release) {
      var asset = pickInstaller(release.assets || []);
      if (!asset) return;
      var version = String(release.tag_name || "").replace(/^v/i, "");
      var label = (version ? "Versión " + version + " · " : "") + fmtSize(asset.size);
      buttons.forEach(function (b) { if (b) b.href = asset.browser_download_url; });
      metas.forEach(function (m) { if (m) m.textContent = label + " · Windows"; });
      if (detail) detail.textContent = "Windows 10 / 11 de 64 bits. " + asset.name + " · " + fmtSize(asset.size) + ".";
      showHash(asset);
    })
    .catch(function () { /* se queda el enlace a la página de versiones */ });
})();
