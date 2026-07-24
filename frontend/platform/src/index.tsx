/* @refresh reload */
import { render } from 'solid-js/web';
import '@sesame/shared/styles/theme.css';
import { App } from './App';

const root = document.getElementById('root');
if (!root) throw new Error('missing #root');
render(() => <App />, root);
