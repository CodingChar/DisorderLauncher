<script lang="ts">
  import './app.css'
</script>

<div class="shell">
  <aside class="sidebar">
    <div class="brand">
      <small>LAUNCHER</small>
      <strong>Disorder</strong>
    </div>

    <nav class="nav">
      <button class="active">
        <span>Inicio</span>
      </button>
      <button>
        <span>Instancias</span>
      </button>
      <button>
        <span>Preferencias</span>
      </button>
    </nav>

    <div class="spacer"></div>

    <div class="profile-box">
      <strong>DisorderLauncher</strong>
      <span>v0.1.0 — Offline Gaming</span>
    </div>
  </aside>

  <main class="main">
    <div class="top">
      <div class="title-block">
        <small>BIENVENIDO</small>
        <h1>DisorderLauncher</h1>
        <p>Launcher de Minecraft Java ligero, modular y seguro.</p>
      </div>
    </div>

    <div class="hero-card">
      <div class="hero-content">
        <span class="kicker">LISTO PARA JUGAR</span>
        <h2>Crea tu primer perfil para empezar</h2>
        <p>DisorderLauncher te permite jugar Minecraft Java con identidad local. Sin cuentas externas, sin complicaciones.</p>
      </div>
    </div>
  </main>
</div>

<style>
  .shell {
    min-height: 100vh;
    display: grid;
    grid-template-columns: 210px minmax(0, 1fr);
  }

  .sidebar {
    background: var(--surface);
    border-right: 1px solid var(--border);
    padding: 20px 14px;
    display: flex;
    flex-direction: column;
    height: 100vh;
    position: sticky;
    top: 0;
  }

  .brand {
    border: 1px solid var(--border-strong);
    border-radius: 0 16px 16px 0;
    padding: 14px 12px;
    margin-bottom: 26px;
  }

  .brand small {
    display: block;
    color: var(--text-secondary);
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    margin-bottom: 4px;
  }

  .brand strong {
    font-family: var(--font-display);
    font-size: 18px;
    letter-spacing: -0.035em;
    font-weight: 700;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .nav button {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    padding: 12px;
    text-align: left;
    border-radius: var(--r-sm);
    display: flex;
    align-items: center;
    gap: 10px;
    transition: 0.18s;
    min-height: 44px;
  }

  .nav button:hover {
    background: var(--surface-alt);
    color: var(--text-primary);
  }

  .nav button.active {
    background: var(--action);
    color: var(--action-text);
    border-radius: 0 10px 10px 0;
    font-weight: 700;
  }

  .spacer {
    flex: 1;
  }

  .profile-box {
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }

  .profile-box strong {
    display: block;
    font-size: 13px;
  }

  .profile-box span {
    color: var(--text-secondary);
    font-size: 11px;
  }

  .main {
    padding: 24px;
    min-width: 0;
    overflow-y: auto;
    height: 100vh;
  }

  .top {
    margin-bottom: 20px;
  }

  .title-block small {
    display: block;
    color: var(--text-secondary);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.18em;
    margin-bottom: 5px;
  }

  .title-block h1 {
    font-size: 34px;
    line-height: 0.95;
  }

  .title-block p {
    margin: 8px 0 0;
    color: var(--text-secondary);
    font-size: 13px;
    max-width: 650px;
  }

  .hero-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 24px 0 24px 0;
    padding: 26px;
    min-height: 280px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    box-shadow: var(--shadow-soft);
  }

  .kicker {
    display: inline-block;
    padding: 6px 9px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .hero-content h2 {
    font-size: 36px;
    line-height: 0.98;
    letter-spacing: -0.06em;
    margin: 18px 0 12px;
    max-width: 660px;
  }

  .hero-content p {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.65;
    max-width: 590px;
  }
</style>
