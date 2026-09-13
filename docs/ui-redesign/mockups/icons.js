/* Minimal 24x24 line icons for navigation + status, shared by all mockups. */
window.ICONS = {
  overview: '<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="2.5" fill="currentColor" stroke="none"/><path d="M12 4v3M12 17v3M4 12h3M17 12h3"/>',
  cats: '<path d="M5 10l1-6 4 3h4l4-3 1 6c0 5-3 8-7 8s-7-3-7-8z"/><circle cx="9.5" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="14.5" cy="12" r="1" fill="currentColor" stroke="none"/><path d="M10 15.5h4"/>',
  build: '<path d="M14 4l6 6-2 2-6-6z"/><path d="M13 9l-8 8 2 2 8-8"/><path d="M17 6l1-1"/>',
  work: '<circle cx="12" cy="12" r="3.5"/><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M18.4 5.6l-2.1 2.1M7.7 16.3l-2.1 2.1"/>',
  stores: '<path d="M4 8l8-4 8 4v9l-8 4-8-4z"/><path d="M4 8l8 4 8-4M12 12v9"/>',
  research: '<path d="M9 3h6M10 3v6l-5 9a2 2 0 002 3h10a2 2 0 002-3l-5-9V3"/><path d="M8 15h8"/>',
  officers: '<path d="M12 3l2.6 5.3 5.9.8-4.3 4.1 1 5.8L12 16.3 6.8 19l1-5.8L3.5 9.1l5.9-.8z"/>',
  shrine: '<path d="M4 7h16M6 7v13M18 7v13M5 11h14M9 20v-6h6v6"/><path d="M4 7l2-3h12l2 3"/>',
  world: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18"/>',
  defense: '<path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z"/><path d="M9 12l2 2 4-4"/>',
  trade: '<path d="M4 8h13l-3-3M20 16H7l3 3"/>',
  routes: '<circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M8 7c6 0 4 10 8 10"/>',
  events: '<path d="M6 16V11a6 6 0 0112 0v5l2 2H4z"/><path d="M10 20a2 2 0 004 0"/>',
  village: '<path d="M6 21V4M6 4h11l-2 4 2 4H6"/>',
  alert: '<path d="M12 3l9 16H3z"/><path d="M12 10v4M12 17v.5"/>',
  clock: '<circle cx="12" cy="12" r="8"/><path d="M12 8v4l3 2"/>',
  check: '<path d="M5 12l4 4 10-10"/>',
  pause: '<path d="M8 5v14M16 5v14" stroke-width="3"/>',
  play: '<path d="M7 5l12 7-12 7z" fill="currentColor" stroke="none"/>',
  pin: '<path d="M12 21s-6-6.5-6-11a6 6 0 0112 0c0 4.5-6 11-6 11z"/><circle cx="12" cy="10" r="2"/>',
  arrow: '<path d="M5 12h14M13 6l6 6-6 6"/>',
  drop: '<path d="M12 3s6 7 6 11a6 6 0 01-12 0c0-4 6-11 6-11z"/>',
  bed: '<path d="M3 18V8M3 12h18v6M3 16h18M7 12V9h5v3"/>',
  search: '<circle cx="11" cy="11" r="6"/><path d="M16 16l5 5"/>',
};
window.icon = function (name, cls) { return `<svg class="ic ${cls || ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[name] || ''}</svg>`; };
window.NAV = ['Overview', 'Cats', 'Build', 'Work', 'Stores', 'Research', 'Officers', 'Shrine', 'World', 'Defense', 'Trade', 'Routes', 'Events', 'Village'];
window.NAV_KEYS = ['O', 'C', 'B', 'W', 'S', 'R', 'F', 'H', 'M', 'D', 'T', 'U', 'E', 'V'];
