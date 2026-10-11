/* @refresh reload */
import { render } from 'solid-js/web';
import '@sesame/shared/styles/theme.css';
import { loadRuntimeConfig } from '@sesame/shared';
import { App } from './App';

const root = document.getElementById('root');
if (!root) throw new Error('missing #root');
// Deployment settings come from /config.json at start-up (one image, every environment).
void loadRuntimeConfig({
  defaultTenant: import.meta.env.VITE_DEFAULT_TENANT,
  defaultClientId: import.meta.env.VITE_DEFAULT_CLIENT_ID,
}).then(() => render(() => <App />, root));
