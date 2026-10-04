// Lets a screen with unsaved work intercept navigation away from it. The
// screen registers a guard that asks the user and resolves true to leave;
// the shell calls `confirmLeave` before switching screens.

type Guard = () => Promise<boolean>;

let guard: Guard | null = null;

export function setLeaveGuard(next: Guard | null) {
  guard = next;
}

export function confirmLeave(): Promise<boolean> {
  return guard ? guard() : Promise.resolve(true);
}
