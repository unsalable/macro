import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './en.json';
import tr from './tr.json';

export const SUPPORTED_LANGUAGES = ['en', 'tr'] as const;
export type Language = (typeof SUPPORTED_LANGUAGES)[number];

export function detectLanguage(): Language {
  const nav = navigator.language.slice(0, 2).toLowerCase();
  return (SUPPORTED_LANGUAGES as readonly string[]).includes(nav) ? (nav as Language) : 'en';
}

void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    tr: { translation: tr },
  },
  lng: detectLanguage(),
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
  returnNull: false,
});

export default i18n;
