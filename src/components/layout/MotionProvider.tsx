import { MotionConfig } from 'motion/react';
import { useEffect, type ReactNode } from 'react';

import { duration, speedFactor, type AnimationQuality } from '@/lib/motion';

interface MotionProviderProps {
  quality: AnimationQuality;
  children: ReactNode;
}

/**
 * One place decides how fast the whole app moves (§46, §68). Both the JS
 * motion tokens and the CSS ones are scaled here, so Radix transitions and
 * motion components always agree.
 */
export function MotionProvider({ quality, children }: MotionProviderProps) {
  const factor = speedFactor(quality);

  useEffect(() => {
    const root = document.documentElement;
    if (factor === 0) {
      root.style.setProperty('--fm-fast', '1ms');
      root.style.setProperty('--fm-normal', '1ms');
    } else {
      root.style.setProperty('--fm-fast', `${Math.round(duration.fast * factor * 1000)}ms`);
      root.style.setProperty('--fm-normal', `${Math.round(duration.normal * factor * 1000)}ms`);
    }
  }, [factor]);

  return (
    <MotionConfig
      reducedMotion={quality === 'low' ? 'always' : 'user'}
      transition={{ duration: duration.normal * (factor || 0.001) }}
    >
      {children}
    </MotionConfig>
  );
}
