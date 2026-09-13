/* Both static pages remain readable without JavaScript. Only explicit choices
   are persisted; automatic detection must follow browser preferences next time. */
(() => {
  const key = 'vc-rs.site-language';
  const supported = value => value === 'ja' || value === 'en';
  const url = new URL(window.location.href);
  const explicit = url.searchParams.get('lang');
  let saved;
  try { saved = window.localStorage.getItem(key); } catch { /* Storage may be blocked. */ }
  const primary = navigator.languages?.[0] || navigator.language || 'en';
  const language = supported(explicit) ? explicit : supported(saved) ? saved
    : /^ja(?:-|$)/i.test(primary) ? 'ja' : 'en';
  if (supported(explicit)) {
    try { window.localStorage.setItem(key, language); } catch { /* Switching still works. */ }
  }
  const target = new URL(language === 'ja' ? './' : './en.html', url);
  target.search = url.search;
  target.hash = url.hash;
  if (document.documentElement.lang !== language) {
    window.location.replace(target.href);
  } else if (supported(explicit)) {
    // Remove the selection parameter so later visits use the saved preference.
    url.searchParams.delete('lang');
    window.history.replaceState(null, '', url.href);
  }
})();
