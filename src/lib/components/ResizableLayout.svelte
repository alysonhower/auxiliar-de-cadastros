<script lang="ts">
  import * as Resizable from "$lib/components/ui/resizable";
  import Render from "./Render.svelte";
  import StatusPanel from "./StatusPanel.svelte";
  import type { ProcessedPage } from '$lib/types';

  let selectedPages: number[] = $state([]);
  let processingPages: number[] = $state([]);
  let processedPages: ProcessedPage[] = $state([]);
</script>

<Resizable.PaneGroup direction="horizontal">
  <Resizable.Pane defaultSize={30}><StatusPanel {processedPages} /></Resizable.Pane>
  <Resizable.Handle />
  <Resizable.Pane
    ><Render
      bind:selectedPages
      bind:processingPages
      bind:processedPages
    /></Resizable.Pane
  >
</Resizable.PaneGroup>