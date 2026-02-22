<script lang="ts">
  import { marked } from 'marked';

  export let content = '';
  export let title = 'プレビュー';

  function renderMarkdown(md: string) {
    if (!md.trim()) return '<p class="text-text-muted italic">内容がありません</p>';
    return marked.parse(md);
  }
</script>

<div class="bg-surface-primary rounded-2xl border border-border-base shadow-sm overflow-hidden flex flex-col h-full">
  <div class="px-4 py-2 bg-surface-secondary border-b border-border-base flex items-center gap-2">
    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-text-muted"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>
    <span class="text-[10px] font-black text-text-muted uppercase tracking-widest">{title}</span>
  </div>
  <div class="p-4 overflow-y-auto flex-1 prose prose-slate dark:prose-invert prose-sm max-w-none prose-headings:font-black prose-h2:text-sm prose-h2:mb-2">
    {@html renderMarkdown(content)}
  </div>
</div>
