import '../../../../src/app.css';
import { mount } from 'svelte';
import ApprovalsDock from '../../../../src/lib/components/ApprovalsDock.svelte';

const main = document.querySelector('main');
if (main) {
  mount(ApprovalsDock, { target: main, props: { hudUrl: 'http://hud' } });
}
