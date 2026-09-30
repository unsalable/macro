import { getVersion } from '@tauri-apps/api/app';
import { disable as disableAutostart, enable as enableAutostart } from '@tauri-apps/plugin-autostart';
import { Database, Download, Gauge, KeyRound, Palette, Settings2 } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { motion } from 'motion/react';
import { useEffect, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { PageShell } from '@/components/layout/PageShell';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { HotkeyInput } from '@/components/ui/hotkey-input';
import { Field, NumberInput } from '@/components/ui/input';
import { Kbd, Separator } from '@/components/ui/misc';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { SwitchRow } from '@/components/ui/switch';
import { cn } from '@/lib/cn';
import { hotkeysEqual } from '@/lib/hotkey';
import { transition } from '@/lib/motion';
import { SUPPORTED_LANGUAGES, type Language } from '@/i18n';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUpdateStore } from '@/stores/updateStore';
import type { AnimationQuality } from '@/lib/motion';
import type { Hotkey, HotkeyMap, ThemeMode } from '@/types';

type Category = 'general' | 'appearance' | 'hotkeys' | 'performance' | 'updates' | 'data';

const CATEGORIES: { id: Category; labelKey: string; icon: LucideIcon }[] = [
  { id: 'general', labelKey: 'settings.general', icon: Settings2 },
  { id: 'appearance', labelKey: 'settings.appearance', icon: Palette },
  { id: 'hotkeys', labelKey: 'settings.hotkeys', icon: KeyRound },
  { id: 'performance', labelKey: 'settings.performance', icon: Gauge },
  { id: 'updates', labelKey: 'settings.updates', icon: Download },
  { id: 'data', labelKey: 'settings.data', icon: Database },
];

const HOTKEY_SLOTS: { key: keyof HotkeyMap; labelKey: string; hintKey: string }[] = [
  { key: 'start', labelKey: 'hotkey.start', hintKey: 'hotkey.startDesc' },
  { key: 'stop', labelKey: 'hotkey.stop', hintKey: 'hotkey.stopDesc' },
  { key: 'pause', labelKey: 'hotkey.pause', hintKey: 'hotkey.pauseDesc' },
  { key: 'emergencyStop', labelKey: 'hotkey.emergency', hintKey: 'hotkey.emergencyDesc' },
];

const QUALITY_LEVELS: AnimationQuality[] = ['high', 'balanced', 'low'];

export function SettingsPage() {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  const update = useSettingsStore((store) => store.update);
  const [category, setCategory] = useState<Category>('general');
  const version = useAppVersion();

  const setHotkey = (slot: keyof HotkeyMap, value: Hotkey | null) => {
    // Two actions sharing a shortcut would make the second one unreachable.
    const clash = HOTKEY_SLOTS.some(
      ({ key }) => key !== slot && value && hotkeysEqual(settings.hotkeys[key], value),
    );
    if (clash) {
      toast.error(t('error.hotkeyTaken'));
      return;
    }
    void update({ hotkeys: { ...settings.hotkeys, [slot]: value } });
  };

  const setAutostart = async (enabled: boolean) => {
    try {
      await (enabled ? enableAutostart() : disableAutostart());
      await update({ startWithWindows: enabled });
    } catch (error) {
      toast.error(t('error.generic'), { description: String(error) });
    }
  };

  return (
    <PageShell title={t('settings.title')}>
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-[200px_minmax(0,1fr)]">
        <nav className="flex flex-col gap-0.5">
          {CATEGORIES.map((item) => (
            <button
              key={item.id}
              type="button"
              onClick={() => setCategory(item.id)}
              className={cn(
                'relative flex h-9 items-center gap-2.5 rounded-[var(--radius-sm)] px-3 text-[13px] font-medium transition-colors',
                category === item.id
                  ? 'text-foreground'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {category === item.id ? (
                <motion.span
                  layoutId="settings-active"
                  transition={transition.spring}
                  className="absolute inset-0 rounded-[var(--radius-sm)] bg-card shadow-[var(--shadow-soft)]"
                />
              ) : null}
              <item.icon className="relative z-10 size-4" />
              <span className="relative z-10">{t(item.labelKey)}</span>
            </button>
          ))}
        </nav>

        <Card>
          <CardContent className="p-5">
            <motion.div
              key={category}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              transition={transition.normal}
              className="flex flex-col"
            >
              {category === 'general' ? (
                <Section blurb={t('settings.generalDesc')}>
                  <Field
                    label={t('settings.language')}
                    hint={t('settings.languageDesc')}
                    className="pb-4"
                  >
                    <Select
                      value={settings.language}
                      onValueChange={(value) => void update({ language: value as Language })}
                    >
                      <SelectTrigger className="max-w-56">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {SUPPORTED_LANGUAGES.map((code) => (
                          <SelectItem key={code} value={code}>
                            {code === 'en' ? 'English' : 'Türkçe'}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </Field>
                  <Separator />
                  <SwitchRow
                    label={t('settings.startWithWindows')}
                    description={t('settings.startWithWindowsDesc')}
                    checked={settings.startWithWindows}
                    onCheckedChange={(checked) => void setAutostart(checked)}
                  />
                  <SwitchRow
                    label={t('settings.startMinimized')}
                    description={t('settings.startMinimizedDesc')}
                    checked={settings.startMinimized}
                    onCheckedChange={(startMinimized) => void update({ startMinimized })}
                  />
                  <SwitchRow
                    label={t('settings.minimizeToTray')}
                    description={t('settings.minimizeToTrayDesc')}
                    checked={settings.closeToTray}
                    onCheckedChange={(closeToTray) => void update({ closeToTray })}
                  />
                  <SwitchRow
                    label={t('settings.showTrayNotifications')}
                    description={t('settings.showTrayNotificationsDesc')}
                    checked={settings.showTrayNotifications}
                    onCheckedChange={(showTrayNotifications) =>
                      void update({ showTrayNotifications })
                    }
                  />
                  <SwitchRow
                    label={t('settings.confirmBeforeDelete')}
                    description={t('settings.confirmBeforeDeleteDesc')}
                    checked={settings.confirmBeforeDelete}
                    onCheckedChange={(confirmBeforeDelete) => void update({ confirmBeforeDelete })}
                  />
                </Section>
              ) : null}

              {category === 'appearance' ? (
                <Section blurb={t('settings.appearanceDesc')}>
                  <Field label={t('settings.theme')} hint={t('settings.themeDesc')}>
                    <Select
                      value={settings.theme}
                      onValueChange={(value) => void update({ theme: value as ThemeMode })}
                    >
                      <SelectTrigger className="max-w-56">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="system">{t('settings.themeSystem')}</SelectItem>
                        <SelectItem value="light">{t('settings.themeLight')}</SelectItem>
                        <SelectItem value="dark">{t('settings.themeDark')}</SelectItem>
                      </SelectContent>
                    </Select>
                  </Field>
                </Section>
              ) : null}

              {category === 'hotkeys' ? (
                <Section blurb={t('settings.hotkeysDesc')}>
                  <div className="flex flex-col divide-y divide-[var(--border)]">
                    {HOTKEY_SLOTS.map(({ key, labelKey, hintKey }) => (
                      <div key={key} className="flex items-center justify-between gap-6 py-3">
                        <span className="flex flex-col gap-0.5">
                          <span className="text-[13px] font-medium">{t(labelKey)}</span>
                          <span className="text-xs leading-relaxed text-muted-foreground">
                            {t(hintKey)}
                          </span>
                        </span>
                        <HotkeyInput
                          value={settings.hotkeys[key]}
                          onChange={(value) => setHotkey(key, value)}
                        />
                      </div>
                    ))}
                    <SwitchRow
                      label={`${t('settings.emergencyStop')} · Esc ×3`}
                      description={t('settings.panicHint')}
                      checked={settings.panicEscapeEnabled}
                      onCheckedChange={(panicEscapeEnabled) => void update({ panicEscapeEnabled })}
                    />
                  </div>
                  <p className="pt-4 text-xs leading-relaxed text-muted-foreground">
                    {t('settings.hotkeyRepeatNote')}
                  </p>
                </Section>
              ) : null}

              {category === 'performance' ? (
                <Section blurb={t('settings.performanceDesc')}>
                  <div className="flex flex-col gap-1 pb-3">
                    <span className="text-[13px] font-medium">
                      {t('settings.animationQuality')}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {t('settings.animationQualityDesc')}
                    </span>
                  </div>

                  {/* The levels are the control. A dropdown would hide what
                      each one costs behind a single word. */}
                  <div className="flex flex-col divide-y divide-[var(--border)] overflow-hidden rounded-[var(--radius-sm)] border border-[var(--border)]">
                    {QUALITY_LEVELS.map((level) => {
                      const current = settings.animationQuality === level;
                      return (
                        <button
                          key={level}
                          type="button"
                          onClick={() => void update({ animationQuality: level })}
                          className={cn(
                            'flex items-start gap-3 px-3.5 py-3 text-left transition-colors',
                            current
                              ? 'bg-[var(--accent-soft)]'
                              : 'bg-transparent hover:bg-[var(--card-muted)]',
                          )}
                        >
                          <span
                            className={cn(
                              'mt-1 size-2 shrink-0 rounded-full',
                              current ? 'bg-accent' : 'bg-[var(--border-strong)]',
                            )}
                          />
                          <span className="flex flex-col gap-0.5">
                            <span
                              className={cn(
                                'text-[13px] font-medium',
                                current ? 'text-accent' : 'text-foreground',
                              )}
                            >
                              {t(`settings.${level}`)}
                            </span>
                            <span className="text-xs leading-relaxed text-muted-foreground">
                              {t(`settings.quality_${level}`)}
                            </span>
                          </span>
                        </button>
                      );
                    })}
                  </div>

                  <p className="pt-4 text-xs leading-relaxed text-muted-foreground">
                    {t('settings.performanceNote')}
                  </p>
                </Section>
              ) : null}

              {category === 'updates' ? (
                <Section blurb={t('settings.updatesDesc')}>
                  <UpdatePanel version={version} />
                  <Separator />
                  <SwitchRow
                    label={t('settings.autoUpdateCheck')}
                    description={t('settings.autoUpdateCheckDesc')}
                    checked={settings.autoUpdateCheck}
                    onCheckedChange={(autoUpdateCheck) => void update({ autoUpdateCheck })}
                  />
                  <p className="pt-2 text-xs leading-relaxed text-muted-foreground">
                    {t('settings.updatePortableNote')}
                  </p>
                </Section>
              ) : null}

              {category === 'data' ? (
                <Section blurb={t('settings.dataDesc')}>
                  <Field
                    label={t('settings.historyLimit')}
                    hint={t('settings.historyLimitDesc')}
                    className="pb-4"
                  >
                    <NumberInput
                      value={settings.historyLimit}
                      min={50}
                      max={5000}
                      onValueChange={(historyLimit) => void update({ historyLimit })}
                      className="max-w-56"
                    />
                  </Field>
                  <Separator />
                  <div className="flex flex-col gap-3 pt-4 text-[13px]">
                    <Detail label={t('settings.dataFolder')} value="%APPDATA%\FlowMacro" />
                    <Detail
                      label={t('settings.version')}
                      value={`${version} · schema ${settings.schemaVersion}`}
                    />
                  </div>
                </Section>
              ) : null}
            </motion.div>
          </CardContent>
        </Card>
      </div>
    </PageShell>
  );
}

/** Every tab opens with one line saying what it is for (§25). */
function Section({ blurb, children }: { blurb: string; children: ReactNode }) {
  return (
    <div className="flex flex-col">
      <p className="pb-4 text-[13px] leading-relaxed text-muted-foreground">{blurb}</p>
      <Separator />
      <div className="flex flex-col pt-4">{children}</div>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-6">
      <span className="text-muted-foreground">{label}</span>
      <Kbd className="font-mono">{value}</Kbd>
    </div>
  );
}

/** The version baked into the bundle, so it can never drift from the build. */
function useAppVersion(): string {
  const [version, setVersion] = useState('—');
  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion('—'));
  }, []);
  return version;
}

/**
 * The whole update flow in one block: what is installed, what is offered, and
 * the single button that does the next thing.
 */
function UpdatePanel({ version }: { version: string }) {
  const { t } = useTranslation();
  const status = useUpdateStore((store) => store.status);
  const available = useUpdateStore((store) => store.version);
  const progress = useUpdateStore((store) => store.progress);
  const error = useUpdateStore((store) => store.error);
  const check = useUpdateStore((store) => store.check);
  const install = useUpdateStore((store) => store.install);

  const busy = status === 'checking' || status === 'downloading' || status === 'ready';

  const message = () => {
    switch (status) {
      case 'checking':
        return t('update.checking');
      case 'available':
        return t('update.available', { version: available });
      case 'downloading':
        return progress === null
          ? t('update.downloading')
          : t('update.downloadingPercent', { percent: Math.round(progress * 100) });
      case 'ready':
        return t('update.restarting');
      case 'current':
        return t('update.upToDate');
      case 'error':
        return error ?? t('error.generic');
      default:
        return t('update.idle');
    }
  };

  return (
    <div className="flex flex-col gap-3 pb-4">
      <div className="flex items-center justify-between gap-6">
        <span className="flex flex-col gap-0.5">
          <span className="text-[13px] font-medium">
            {t('settings.currentVersion', { version })}
          </span>
          <span
            className={cn(
              'text-xs leading-relaxed',
              status === 'error' ? 'text-[var(--danger)]' : 'text-muted-foreground',
            )}
          >
            {message()}
          </span>
        </span>

        {status === 'available' ? (
          <Button onClick={() => void install()} disabled={busy}>
            {t('update.install')}
          </Button>
        ) : (
          <Button variant="secondary" onClick={() => void check()} disabled={busy}>
            {t('update.check')}
          </Button>
        )}
      </div>

      {status === 'downloading' && progress !== null ? (
        <div className="h-1 overflow-hidden rounded-full bg-[var(--card-muted)]">
          <div
            className="h-full rounded-full bg-accent transition-[width] duration-200"
            style={{ width: `${Math.round(progress * 100)}%` }}
          />
        </div>
      ) : null}
    </div>
  );
}
