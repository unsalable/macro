import { createContext, useContext } from 'react';

/**
 * One ordering for the whole app: the sidebar renders in this order and the
 * page transition takes its direction from it, so moving down the list always
 * pushes the page up and moving back up reverses it (§29, §67).
 */
export const NAV_PATHS = [
  '/',
  '/macros',
  '/recorder',
  '/mouse',
  '/keyboard',
  '/profiles',
  '/history',
  '/settings',
] as const;

export type NavPath = (typeof NAV_PATHS)[number];

/** Longest matching prefix, so `/macros/abc` still counts as `/macros`. */
export function navIndex(pathname: string): number {
  let best = 0;
  let bestLength = -1;
  NAV_PATHS.forEach((path, index) => {
    const matches = path === '/' ? pathname === '/' : pathname.startsWith(path);
    if (matches && path.length > bestLength) {
      best = index;
      bestLength = path.length;
    }
  });
  return best;
}

/** +1 when navigating further down the sidebar, -1 when coming back up. */
export const NavDirectionContext = createContext(1);

export function useNavDirection(): number {
  return useContext(NavDirectionContext);
}
