// Buddy in 3D (buddy3d.bundle.js, three.js, ~730 kB) loads once your mail is on screen. As a plain <script> in the
// page it held back everything after it (calendar, workspaces, shortcuts) and the page's "ready". buddy-ui.js mounts
// him when "buddy3d-ready" fires.
'use strict';
(() => {
  const go = () => { const s = document.createElement('script'); s.src = 'buddy3d.bundle.js'; s.async = true; document.body.appendChild(s); };
  const later = () => (window.requestIdleCallback ? requestIdleCallback(go, { timeout: 1200 }) : setTimeout(go, 300));
  if (document.readyState === 'loading') addEventListener('DOMContentLoaded', later, { once: true }); else later();
})();
