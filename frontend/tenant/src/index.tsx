/* @refresh reload */
import { render } from 'solid-js/web';
import '@sesame/shared/styles/theme.css';
import { loadRuntimeConfig } from '@sesame/shared';
import { App } from './App';

const root = document.getElementById('root');
if (!root) throw new Error('missing #root');
// Deployment settings come from /config.json at start-up (one image, every environment).
void loadRuntimeConfig({
  authBaseUrl: import.meta.env.VITE_AUTH_BASE_URL,
  tenantId: import.meta.env.VITE_TENANT_ID,
  environment: import.meta.env.VITE_ENVIRONMENT,
}).then(() => render(() => <App />, root));
