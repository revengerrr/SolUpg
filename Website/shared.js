// Reveal on scroll
(() => {
  const io = new IntersectionObserver((entries) => {
    entries.forEach(e => {
      if (e.isIntersecting) {
        e.target.classList.add('in');
        io.unobserve(e.target);
      }
    });
  }, { threshold: 0.1, rootMargin: '0px 0px -40px 0px' });
  document.querySelectorAll('.reveal').forEach(el => io.observe(el));
})();

// Hamburger menu toggle
(() => {
  const btn = document.querySelector('.nav-hamburger');
  const links = document.querySelector('.nav-links');
  if (!btn || !links) return;
  btn.addEventListener('click', () => {
    const open = links.classList.toggle('open');
    btn.setAttribute('aria-expanded', open);
  });
  document.addEventListener('click', (e) => {
    if (!btn.contains(e.target) && !links.contains(e.target)) {
      links.classList.remove('open');
      btn.setAttribute('aria-expanded', 'false');
    }
  });
})();

// Tweaks host integration
(() => {
  const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
    "accentHue": 275,
    "density": "default",
    "showArchLabels": true
  }/*EDITMODE-END*/;

  const state = { ...TWEAK_DEFAULTS };
  const panel = document.getElementById('tweaks-panel');

  function applyState() {
    document.documentElement.style.setProperty('--accent', `oklch(0.62 0.26 ${state.accentHue})`);
    document.documentElement.style.setProperty('--accent-soft', `oklch(0.62 0.26 ${state.accentHue} / 0.12)`);
    document.documentElement.style.setProperty('--accent-line', `oklch(0.62 0.26 ${state.accentHue} / 0.35)`);
    document.documentElement.dataset.density = state.density;
    document.documentElement.dataset.archLabels = state.showArchLabels;
  }
  applyState();

  window.addEventListener('message', (e) => {
    if (!e.data || typeof e.data !== 'object') return;
    if (e.data.type === '__activate_edit_mode') {
      if (panel) panel.classList.add('open');
    } else if (e.data.type === '__deactivate_edit_mode') {
      if (panel) panel.classList.remove('open');
    }
  });
  try { window.parent.postMessage({ type: '__edit_mode_available' }, '*'); } catch {}

  if (panel) {
    const hue = panel.querySelector('#tw-hue');
    const hueV = panel.querySelector('#tw-hue-v');
    const dens = panel.querySelector('#tw-density');
    const labs = panel.querySelector('#tw-labels');
    if (hue) {
      hue.value = state.accentHue;
      if (hueV) hueV.textContent = state.accentHue;
      hue.addEventListener('input', () => {
        state.accentHue = +hue.value;
        if (hueV) hueV.textContent = hue.value;
        applyState();
        try { window.parent.postMessage({ type: '__edit_mode_set_keys', edits: { accentHue: state.accentHue } }, '*'); } catch {}
      });
    }
    if (dens) {
      dens.value = state.density;
      dens.addEventListener('change', () => {
        state.density = dens.value;
        applyState();
        try { window.parent.postMessage({ type: '__edit_mode_set_keys', edits: { density: state.density } }, '*'); } catch {}
      });
    }
    if (labs) {
      labs.checked = state.showArchLabels;
      labs.addEventListener('change', () => {
        state.showArchLabels = labs.checked;
        applyState();
        try { window.parent.postMessage({ type: '__edit_mode_set_keys', edits: { showArchLabels: state.showArchLabels } }, '*'); } catch {}
      });
    }
  }

  // Density
  const style = document.createElement('style');
  style.textContent = `
    html[data-density="compact"] .section-body { padding-top: 20px; padding-bottom: 48px; }
    html[data-density="compact"] .section-head { padding-top: 32px; }
    html[data-density="spacious"] .section-body { padding-top: 48px; padding-bottom: 96px; }
    html[data-density="spacious"] .section-head { padding-top: 72px; }
    html[data-arch-labels="false"] .arch-label { display: none !important; }
  `;
  document.head.appendChild(style);
})();
