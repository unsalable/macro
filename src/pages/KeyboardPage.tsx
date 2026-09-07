import { Play, Plus } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';

import { KeyboardVisualizer } from '@/components/keyboard/KeyboardVisualizer';
import { PageShell } from '@/components/layout/PageShell';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Field, NumberInput } from '@/components/ui/input';
import { Kbd } from '@/components/ui/misc';
import { Switch } from '@/components/ui/switch';
import { createAction } from '@/lib/actions';
import { formatKeyCode } from '@/lib/hotkey';
import { engine as engineApi, macros as macroApi } from '@/services/tauri';
import { useMacroStore } from '@/stores/macroStore';
import { MODIFIERS, type MacroAction, type Modifier } from '@/types';

type KeyAction = Extract<MacroAction, { type: 'key' }>;

export function KeyboardPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const macros = useMacroStore((store) => store.macros);
  const save = useMacroStore((store) => store.save);

  const [selected, setSelected] = useState<string | null>('KeyA');
  const [modifiers, setModifiers] = useState<Modifier[]>([]);
  const [holdMs, setHoldMs] = useState(0);

  const used = useMemo(
    () =>
      macros
        .flatMap((item) => item.actions)
        .filter((action) => action.type === 'key')
        .map((action) => action.code),
    [macros],
  );

  const buildAction = (): KeyAction | null => {
    if (!selected) return null;
    const base = createAction('key');
    return { ...base, type: 'key', action: 'press', code: selected, modifiers, durationMs: holdMs };
  };

  const test = async () => {
    const action = buildAction();
    if (!action) return;
    try {
      await engineApi.testAction(action);
    } catch (error) {
      toast.error(t('error.generic'), { description: String(error) });
    }
  };

  const createMacro = async () => {
    const action = buildAction();
    if (!action) return;
    const label = [...modifiers.map((item) => item.toUpperCase()), formatKeyCode(action.code)].join('+');
    const draft = await macroApi.create(`${t('keyboard.title')} · ${label}`);
    const saved = await save({ ...draft, actions: [action] });
    void navigate(`/macros/${saved.id}`);
  };

  return (
    <PageShell
      title={t('keyboard.title')}
      description={t('keyboard.subtitle')}
      actions={
        <>
          <Button variant="ghost" disabled={!selected} onClick={() => void test()}>
            <Play />
            {t('common.test')}
          </Button>
          <Button variant="accent" disabled={!selected} onClick={() => void createMacro()}>
            <Plus />
            {t('macros.new')}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-6">
        <Card>
          <CardContent className="overflow-x-auto p-6">
            <KeyboardVisualizer selected={selected} onSelect={setSelected} used={used} />
          </CardContent>
        </Card>

        <div className="grid grid-cols-1 gap-5 lg:grid-cols-2">
          <Card>
            <CardContent className="flex flex-col gap-4 p-5">
              <span className="text-[13px] font-semibold uppercase tracking-wide text-muted-foreground">
                {t('keyboard.combo')}
              </span>

              <div className="flex flex-wrap items-center gap-2">
                {MODIFIERS.map((modifier) => (
                  <label
                    key={modifier}
                    className="flex items-center gap-2 rounded-[var(--radius-sm)] border border-[var(--border)] px-3 py-1.5"
                  >
                    <Switch
                      checked={modifiers.includes(modifier)}
                      onCheckedChange={(checked) =>
                        setModifiers((current) =>
                          checked
                            ? [...current, modifier]
                            : current.filter((item) => item !== modifier),
                        )
                      }
                    />
                    <span className="text-[13px]">{modifier.toUpperCase()}</span>
                  </label>
                ))}
              </div>

              <div className="flex items-center gap-1.5 pt-1">
                {modifiers.map((modifier) => (
                  <Kbd key={modifier}>{modifier.toUpperCase()}</Kbd>
                ))}
                {selected ? <Kbd>{formatKeyCode(selected)}</Kbd> : null}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardContent className="flex flex-col gap-4 p-5">
              <Field label={t('mouse.hold')} hint={t('keyboard.holdHint')}>
                <NumberInput value={holdMs} min={0} suffix="ms" onValueChange={setHoldMs} />
              </Field>
            </CardContent>
          </Card>
        </div>
      </div>
    </PageShell>
  );
}
