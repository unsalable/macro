import {
  ChevronLeft,
  Clock,
  Home,
  Keyboard,
  Layers,
  Mouse,
  Settings,
  Circle,
  Zap,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { motion } from 'motion/react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { NavLink, useLocation } from 'react-router-dom';

import { Tooltip } from '@/components/ui/tooltip';
import { cn } from '@/lib/cn';
import { transition } from '@/lib/motion';
import { NAV_PATHS, navIndex, type NavPath } from '@/lib/nav';
import { useUiStore } from '@/stores/uiStore';

interface NavItem {
  labelKey: string;
  icon: LucideIcon;
}

/** Keyed by path so the order lives in one place — `NAV_PATHS`, which the
    page transition reads too (§29). */
const ITEMS: Record<NavPath, NavItem> = {
  '/': { labelKey: 'nav.home', icon: Home },
  '/macros': { labelKey: 'nav.macros', icon: Zap },
  '/recorder': { labelKey: 'nav.recorder', icon: Circle },
  '/mouse': { labelKey: 'nav.mouse', icon: Mouse },
  '/keyboard': { labelKey: 'nav.keyboard', icon: Keyboard },
  '/profiles': { labelKey: 'nav.profiles', icon: Layers },
  '/history': { labelKey: 'nav.history', icon: Clock },
  '/settings': { labelKey: 'nav.settings', icon: Settings },
};

export function Sidebar() {
  const { t } = useTranslation();
  const location = useLocation();
  const collapsed = useUiStore((store) => store.sidebarCollapsed);
  const toggle = useUiStore((store) => store.toggleSidebar);
  const [hovered, setHovered] = useState<string | null>(null);

  const activePath = NAV_PATHS[navIndex(location.pathname)];

  return (
    <motion.nav
      animate={{ width: collapsed ? 64 : 216 }}
      transition={transition.spring}
      className="flex shrink-0 flex-col justify-between border-r border-[var(--border)] bg-[var(--card-muted)] py-3"
      onMouseLeave={() => setHovered(null)}
    >
      <ul className="flex flex-col gap-0.5 px-2">
        {NAV_PATHS.map((path) => {
          const item = ITEMS[path];
          const active = activePath === path;
          const label = t(item.labelKey);
          const link = (
            <NavLink
              to={path}
              onMouseEnter={() => setHovered(path)}
              className={cn(
                'relative flex h-9 items-center gap-3 rounded-[var(--radius-sm)] px-2.5 text-[13px] font-medium transition-colors',
                active ? 'text-foreground' : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {/* Hover wash and active bar are separate layers so they can
                  slide independently rather than fighting each other (§28). */}
              {hovered === path && !active ? (
                <motion.span
                  layoutId="sidebar-hover"
                  transition={transition.spring}
                  className="absolute inset-0 rounded-[var(--radius-sm)] bg-secondary"
                />
              ) : null}
              {active ? (
                <motion.span
                  layoutId="sidebar-active"
                  transition={transition.spring}
                  className="absolute inset-0 rounded-[var(--radius-sm)] bg-card shadow-[var(--shadow-soft)]"
                />
              ) : null}
              {active ? (
                <motion.span
                  layoutId="sidebar-indicator"
                  transition={transition.spring}
                  className="absolute left-0 top-1/2 h-4 w-[3px] -translate-y-1/2 rounded-full bg-accent"
                />
              ) : null}

              <item.icon className="relative z-10 size-4 shrink-0" />
              {!collapsed ? (
                <motion.span
                  initial={{ opacity: 0, x: -4 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={transition.fast}
                  className="relative z-10 truncate"
                >
                  {label}
                </motion.span>
              ) : null}
            </NavLink>
          );

          return (
            <li key={path}>
              {collapsed ? (
                <Tooltip label={label} side="right">
                  {link}
                </Tooltip>
              ) : (
                link
              )}
            </li>
          );
        })}
      </ul>

      <div className="px-2">
        <button
          type="button"
          onClick={toggle}
          className={cn(
            'flex h-9 w-full items-center gap-3 rounded-[var(--radius-sm)] px-2.5 text-[13px]',
            'text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground',
          )}
        >
          <motion.span animate={{ rotate: collapsed ? 180 : 0 }} transition={transition.spring}>
            <ChevronLeft className="size-4" />
          </motion.span>
          {!collapsed ? <span className="truncate">{t('nav.collapse')}</span> : null}
        </button>
      </div>
    </motion.nav>
  );
}
