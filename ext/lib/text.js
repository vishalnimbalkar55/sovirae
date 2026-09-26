// Text helpers shared by the content script, popup, and tests.

(function (root) {
  const MAX_TEXT = 200000; // UTF-16 code units, the app's limit (spec §9.1).

  /** Tidies text read from the page without changing its words. */
  function tidy(text) {
    return String(text || '')
      .replace(/\r\n?/g, '\n')
      .replace(/[   ]/g, ' ')
      .replace(/[ \t\f\v]+/g, ' ')
      .replace(/ *\n */g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

  function wordCount(text) {
    const m = String(text || '').match(/[\p{L}\p{N}][\p{L}\p{N}'’-]*/gu);
    return m ? m.length : 0;
  }

  /** "Under 1 min", "About 4 min" at 180 words per minute (spec §10.4). */
  function estimate(words, rate = 1) {
    const minutes = words / (180 * (rate > 0 ? rate : 1));
    if (words === 0) return '';
    if (minutes < 1) return 'Under 1 min';
    if (minutes < 60) return `About ${Math.round(minutes)} min`;
    return `About ${Math.floor(minutes / 60)} h ${Math.round(minutes % 60)} min`;
  }

  /** The page's language, for choosing a matching voice. */
  function pageLanguage(doc) {
    const tag = (doc && doc.documentElement && doc.documentElement.lang) || '';
    return /^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*$/.test(tag) ? tag : null;
  }

  /** Cuts at the limit, preferring the end of a sentence. */
  function truncate(text, limit = MAX_TEXT) {
    if (text.length <= limit) return text;
    let cut = text.slice(0, limit);
    if (/[\uD800-\uDBFF]$/.test(cut)) cut = cut.slice(0, -1);
    const end = Math.max(cut.lastIndexOf('. '), cut.lastIndexOf('.\n'), cut.lastIndexOf('? '), cut.lastIndexOf('! '));
    return end > limit / 2 ? cut.slice(0, end + 1) : cut;
  }

  const api = { MAX_TEXT, tidy, wordCount, estimate, pageLanguage, truncate };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.SoviraeText = api;
})(typeof globalThis !== 'undefined' ? globalThis : this);
