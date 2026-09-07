import { useTranslation } from 'react-i18next';

import { HotkeyInput } from '@/components/ui/hotkey-input';
import { Field, Input, NumberInput } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { CpsControl } from '@/components/mouse/CpsControl';
import { MOUSE_BUTTONS, type MacroAction, type MouseButton } from '@/types';

interface ActionFieldsProps {
  action: MacroAction;
  onChange: (next: MacroAction) => void;
}

/** The expanded body of an action card: only the fields that action needs. */
export function ActionFields({ action, onChange }: ActionFieldsProps) {
  const { t } = useTranslation();
  const patch = (values: Partial<MacroAction>) =>
    onChange({ ...action, ...values } as MacroAction);

  switch (action.type) {
    case 'mouse_click':
      return (
        <div className="grid grid-cols-2 gap-4">
          <Field label={t('mouse.button')}>
            <Select
              value={action.button}
              onValueChange={(value) => patch({ button: value as MouseButton })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {MOUSE_BUTTONS.map((button) => (
                  <SelectItem key={button} value={button}>
                    {t(`mouse.${button}`)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          <Field label={t('mouse.clickMode')}>
            <Select
              value={action.mode}
              onValueChange={(value) => patch({ mode: value as 'single' | 'double' | 'hold' })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="single">{t('mouse.single')}</SelectItem>
                <SelectItem value="double">{t('mouse.double')}</SelectItem>
                <SelectItem value="hold">{t('mouse.hold')}</SelectItem>
              </SelectContent>
            </Select>
          </Field>

          <Field label={t('action.count')}>
            <NumberInput
              value={action.count}
              min={1}
              max={100000}
              onValueChange={(count) => patch({ count })}
            />
          </Field>

          {action.mode === 'hold' ? (
            <Field label={t('mouse.hold')}>
              <NumberInput
                value={action.durationMs}
                min={0}
                suffix="ms"
                onValueChange={(durationMs) => patch({ durationMs })}
              />
            </Field>
          ) : null}

          <div className="col-span-2">
            <CpsControl
              intervalMs={action.intervalMs}
              onIntervalChange={(intervalMs) => patch({ intervalMs })}
            />
          </div>
        </div>
      );

    case 'mouse_move':
      return (
        <div className="grid grid-cols-2 gap-4">
          <Field label={t('action.mode')}>
            <Select
              value={action.mode}
              onValueChange={(value) => patch({ mode: value as 'absolute' | 'relative' })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="absolute">{t('action.absolute')}</SelectItem>
                <SelectItem value="relative">{t('action.relative')}</SelectItem>
              </SelectContent>
            </Select>
          </Field>

          <Field label={t('action.curve')}>
            <Select
              value={action.curve}
              onValueChange={(value) => patch({ curve: value as 'linear' | 'smooth' })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="smooth">{t('action.smooth')}</SelectItem>
                <SelectItem value="linear">{t('action.linear')}</SelectItem>
              </SelectContent>
            </Select>
          </Field>

          <Field label="X">
            <NumberInput value={action.x} min={-100000} onValueChange={(x) => patch({ x })} />
          </Field>
          <Field label="Y">
            <NumberInput value={action.y} min={-100000} onValueChange={(y) => patch({ y })} />
          </Field>

          <Field label={t('action.duration')} className="col-span-2">
            <NumberInput
              value={action.durationMs}
              min={0}
              suffix="ms"
              onValueChange={(durationMs) => patch({ durationMs })}
            />
          </Field>
        </div>
      );

    case 'mouse_scroll':
      return (
        <div className="grid grid-cols-2 gap-4">
          <Field label={t('action.direction')}>
            <Select
              value={action.direction}
              onValueChange={(value) => patch({ direction: value as 'up' | 'down' })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="up">{t('action.up')}</SelectItem>
                <SelectItem value="down">{t('action.down')}</SelectItem>
              </SelectContent>
            </Select>
          </Field>
          <Field label={t('action.amount')}>
            <NumberInput
              value={action.amount}
              min={1}
              max={1000}
              onValueChange={(amount) => patch({ amount })}
            />
          </Field>
          <Field label={t('mouse.interval')} className="col-span-2">
            <NumberInput
              value={action.intervalMs}
              min={1}
              suffix="ms"
              onValueChange={(intervalMs) => patch({ intervalMs })}
            />
          </Field>
        </div>
      );

    case 'key':
      return (
        <div className="grid grid-cols-2 gap-4">
          <Field label={t('keyboard.pickKey')} className="col-span-2">
            <HotkeyInput
              value={{ code: action.code, modifiers: action.modifiers }}
              onChange={(hotkey) =>
                patch({ code: hotkey?.code ?? 'KeyA', modifiers: hotkey?.modifiers ?? [] })
              }
            />
          </Field>

          <Field label={t('action.phase')}>
            <Select
              value={action.action}
              onValueChange={(value) => patch({ action: value as 'press' | 'down' | 'up' })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="press">{t('action.press')}</SelectItem>
                <SelectItem value="down">{t('action.keyDown')}</SelectItem>
                <SelectItem value="up">{t('action.keyUp')}</SelectItem>
              </SelectContent>
            </Select>
          </Field>

          <Field label={t('mouse.hold')} hint={t('keyboard.holdHint')}>
            <NumberInput
              value={action.durationMs}
              min={0}
              suffix="ms"
              onValueChange={(durationMs) => patch({ durationMs })}
            />
          </Field>
        </div>
      );

    case 'text':
      return (
        <div className="grid grid-cols-1 gap-4">
          <Field label={t('action.textValue')}>
            <Input
              value={action.value}
              maxLength={4000}
              onChange={(event) => patch({ value: event.target.value })}
            />
          </Field>
          <Field label={t('mouse.interval')}>
            <NumberInput
              value={action.perCharDelayMs}
              min={0}
              max={5000}
              suffix="ms"
              onValueChange={(perCharDelayMs) => patch({ perCharDelayMs })}
            />
          </Field>
        </div>
      );

    case 'delay':
      return (
        <Field label={t('action.duration')}>
          <NumberInput
            value={action.durationMs}
            min={0}
            suffix="ms"
            onValueChange={(durationMs) => patch({ durationMs })}
          />
        </Field>
      );

    case 'repeat':
      return (
        <Field label={t('editor.loopCount')}>
          <NumberInput
            value={action.times}
            min={1}
            max={100000}
            onValueChange={(times) => patch({ times })}
          />
        </Field>
      );
  }
}
