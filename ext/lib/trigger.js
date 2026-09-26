// Long-press trigger state machine (spec §12.3). Pure logic, no DOM, so it
// runs in content scripts and in Node tests.
//
// Idle → Holding → Armed (latched). Any other key while holding cancels and
// leaves the page's own shortcut alone.

(function (root) {
  const KEY_FOR = { alt: 'Alt', control: 'Control' };

  function createTrigger({ trigger = 'alt', holdMs = 450, now = () => Date.now() } = {}) {
    let state = 'idle';
    let startedAt = 0;

    function reset() {
      state = 'idle';
      startedAt = 0;
    }

    return {
      get state() {
        return state;
      },
      configure(opts) {
        if (opts.trigger) trigger = opts.trigger;
        if (opts.holdMs) holdMs = Math.min(1000, Math.max(300, opts.holdMs));
        reset();
      },
      /**
       * Returns 'hold-start' when a hold begins, 'cancel' when a pending hold
       * is abandoned, otherwise null. `ctx` carries flags the DOM layer knows.
       */
      keydown(e, ctx = {}) {
        const wanted = KEY_FOR[trigger];
        if (!wanted || state === 'armed') return null;
        if (e.repeat || e.isComposing || ctx.editable) return state === 'holding' && !e.repeat ? (reset(), 'cancel') : null;
        if (e.altGraph) return state === 'holding' ? (reset(), 'cancel') : null;
        if (e.key === wanted) {
          const others = ['shiftKey', 'metaKey', ...(wanted === 'Alt' ? ['ctrlKey'] : ['altKey'])].some((m) => e[m]);
          if (others) return null;
          if (state === 'idle') {
            state = 'holding';
            startedAt = now();
            return 'hold-start';
          }
          return null;
        }
        // Another key during the hold is a normal shortcut (Ctrl+C, Alt+Tab…).
        if (state === 'holding') {
          reset();
          return 'cancel';
        }
        return null;
      },
      keyup(e) {
        if (state === 'holding' && e.key === KEY_FOR[trigger]) {
          reset();
          return 'cancel';
        }
        return null;
      },
      /** Pointer presses during a hold mean Ctrl+click and the like. */
      pointerdown() {
        if (state === 'holding') {
          reset();
          return 'cancel';
        }
        return null;
      },
      /** Called from a timer; arms once the hold has lasted long enough. */
      tick() {
        if (state === 'holding' && now() - startedAt >= holdMs) {
          state = 'armed';
          return 'armed';
        }
        return null;
      },
      /** Arms directly (keyboard command or button). */
      arm() {
        state = 'armed';
        return 'armed';
      },
      disarm() {
        reset();
      },
      get holdMs() {
        return holdMs;
      },
    };
  }

  const api = { createTrigger };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.SoviraeTrigger = api;
})(typeof globalThis !== 'undefined' ? globalThis : this);
