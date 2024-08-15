<script lang="ts">
  import * as Resizable from '$lib/components/ui/resizable';
  import Render from './Render.svelte';
  import StatusPanel from './StatusPanel.svelte';

  let selectedPages: number[] = $state([]);
  let processingPages: number[] = $state([]);
</script>

<Resizable.PaneGroup direction="horizontal">
  <Resizable.Pane><StatusPanel {selectedPages} /></Resizable.Pane>
  <Resizable.Handle />
  <Resizable.Pane><Render bind:selectedPages bind:processingPages /></Resizable.Pane>
</Resizable.PaneGroup>
