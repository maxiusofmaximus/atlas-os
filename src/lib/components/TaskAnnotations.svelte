<!-- RFC 67 §20 H-05 — task-level notes (F11-b). Lists the `task_annotations`
     rows for a task/run (`GET /task/{id}/annotation`) and adds one
     (`POST /task/{id}/annotation`, body `{body, author, file_path?, line_no?}`).
     An empty body is the backend's 422 and is surfaced inline, never swallowed.
     The task id is the agent run id on the Agent Card. -->

<script lang="ts">
  import { onMount } from 'svelte';

  interface TaskAnnotation {
    id: string;
    task_id: string;
    file_path: string | null;
    line_no: number | null;
    body: string;
    author: string;
    created_at: string;
  }

  interface Props {
    hudUrl?: string | null;
    taskId?: string | null;
    author?: string;
  }

  const { hudUrl, taskId = null, author = 'operator' }: Props = $props();

  let rows = $state<TaskAnnotation[]>([]);
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let draft = $state('');
  let formError = $state<string | null>(null);
  let posting = $state(false);

  function endpoint(): string | null {
    if (!hudUrl || !taskId) return null;
    return `${hudUrl.replace(/\/$/, '')}/task/${encodeURIComponent(taskId)}/annotation`;
  }

  async function load(): Promise<void> {
    const url = endpoint();
    if (!url) {
      loading = false;
      return;
    }
    loading = true;
    loadError = null;
    try {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      rows = (await res.json()) as TaskAnnotation[];
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  async function submit(): Promise<void> {
    const url = endpoint();
    if (!url) return;
    formError = null;
    posting = true;
    try {
      const res = await fetch(url, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ body: draft, author }),
      });
      if (res.status === 422) {
        formError = 'comment body must not be empty (422)';
        return;
      }
      if (!res.ok) {
        formError = `could not save note (${res.status})`;
        return;
      }
      draft = '';
      await load();
    } catch (err) {
      formError = err instanceof Error ? err.message : String(err);
    } finally {
      posting = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<section class="task-notes" aria-label="Task notes">
  <header class="notes-head">
    <h4>Task notes</h4>
    <span class="count">{rows.length}</span>
  </header>

  {#if loading && rows.length === 0}
    <div class="loading" aria-busy="true" aria-label="loading task notes">
      <span class="skel"></span>
      <span class="skel short"></span>
    </div>
  {:else if loadError}
    <div class="error" role="alert">
      <span>could not load task notes</span>
      <button type="button" onclick={() => void load()}>Retry</button>
    </div>
  {:else if rows.length === 0}
    <p class="empty">No task notes yet.</p>
  {:else}
    <ul class="notes">
      {#each rows as row (row.id)}
        <li>
          <span class="author">{row.author}</span>
          <span class="body">{row.body}</span>
        </li>
      {/each}
    </ul>
  {/if}

  <form
    class="note-form"
    onsubmit={(event) => {
      event.preventDefault();
      void submit();
    }}
  >
    <textarea
      class="draft"
      aria-label="New task note"
      placeholder="Add a note…"
      rows="2"
      bind:value={draft}
    ></textarea>
    <div class="form-row">
      <button type="submit" class="add" disabled={posting}>
        {posting ? 'Saving…' : 'Add note'}
      </button>
      {#if formError}<span class="form-error" role="alert">{formError}</span>{/if}
    </div>
  </form>
</section>

<style>
  .task-notes {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    border-top: 1px solid var(--a-border);
    padding-top: 0.55rem;
  }
  .notes-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .notes-head h4 {
    margin: 0;
    font-size: 0.85rem;
    color: var(--a-text);
  }
  .count {
    font-size: 0.72rem;
    color: var(--a-text-faint);
  }
  .loading {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .skel {
    display: block;
    height: 0.75rem;
    border-radius: 4px;
    background: color-mix(in srgb, var(--a-text-faint) 22%, transparent);
  }
  .skel.short {
    width: 60%;
  }
  .error {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.8rem;
    color: var(--a-err);
  }
  .empty {
    margin: 0;
    font-size: 0.8rem;
    color: var(--a-text-muted);
  }
  .notes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    max-height: 10rem;
    overflow-y: auto;
  }
  .notes li {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    background: var(--a-bg);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    font-size: 0.78rem;
  }
  .notes .author {
    color: var(--a-primary);
    flex: 0 0 auto;
  }
  .notes .body {
    color: var(--a-text);
    word-break: break-word;
  }
  .note-form {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .draft {
    width: 100%;
    resize: vertical;
    background: var(--a-bg);
    color: var(--a-text);
    border: 1px solid var(--a-border-ui);
    border-radius: 4px;
    padding: 0.35rem 0.5rem;
    font: inherit;
    font-size: 0.8rem;
  }
  .draft:focus-visible,
  .add:focus-visible,
  .error button:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .form-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .add {
    font-size: 0.78rem;
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    border: 1px solid var(--a-border-ui);
    background: var(--a-surface-2);
    color: var(--a-text);
    cursor: pointer;
  }
  .add:hover {
    border-color: var(--a-primary);
  }
  .add[disabled] {
    opacity: 0.6;
    cursor: default;
  }
  .error button {
    font-size: 0.75rem;
    padding: 0.15rem 0.55rem;
    border-radius: 4px;
    border: 1px solid var(--a-border-ui);
    background: var(--a-surface-2);
    color: var(--a-text);
    cursor: pointer;
  }
  .form-error {
    font-size: 0.75rem;
    color: var(--a-err);
  }
</style>
