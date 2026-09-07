import React from 'react';
import ReactDOM from 'react-dom/client';
import './styles/theme.css';
import './i18n';
import App from './App';

/**
 * Mouse 4 and 5 are back/forward in the webview. A clicker bound to one of
 * them would otherwise walk this window through its own history a hundred
 * times a second whenever it happens to be the focused one — which is exactly
 * when the user is watching, and reads as the app tearing itself apart (§61).
 */
const blockHistoryButtons = (event: MouseEvent) => {
  if (event.button === 3 || event.button === 4) {
    event.preventDefault();
    event.stopPropagation();
  }
};
for (const type of ['mousedown', 'mouseup', 'auxclick', 'click'] as const) {
  window.addEventListener(type, blockHistoryButtons, { capture: true });
}

const container = document.getElementById('root');
if (!container) throw new Error('Root element missing');

ReactDOM.createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
