<script lang="ts">
  import * as Resizable from "$lib/components/ui/resizable";
  import Render from "./Render.svelte";
  import StatusPanel from "./StatusPanel.svelte";
  import type { ProcessingPage, ProcessedDocument } from "$lib/types";

  let selectedPages: number[] = $state([]);
  let processingPages: ProcessingPage[] = $state([]);
  let processedDocuments: ProcessedDocument[] = $state([]);
</script>

<Resizable.PaneGroup direction="horizontal">
  <Resizable.Pane defaultSize={30}>
    <StatusPanel {processedDocuments} {processingPages} />
  </Resizable.Pane>
  <Resizable.Handle />
  <Resizable.Pane>
    <Render
      bind:selectedPages
      bind:processingPages
      bind:processedDocuments
    />
  </Resizable.Pane>
</Resizable.PaneGroup>