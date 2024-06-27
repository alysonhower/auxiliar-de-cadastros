<script lang="ts">
  import * as Resizable from '$lib/components/ui/resizable';
  import Render from './Render.svelte';
  import StatusPanel from './StatusPanel.svelte';

  // Await fix Svelte5: Potential Ownership False Positive 'mutated a value owned...This is strongly discouraged...' #10649
  
  // let selectedPages: number[] = $state([]);
  // let processingPages: number[] = $state([]);
</script>

<Resizable.PaneGroup direction="horizontal">
  <!-- <Resizable.Pane><StatusPanel {selectedPages} /></Resizable.Pane> -->
  <Resizable.Pane><StatusPanel /></Resizable.Pane>
  <Resizable.Handle />
  <!-- <Resizable.Pane><Render {selectedPages} {processingPages} /></Resizable.Pane> -->
  <Resizable.Pane><Render /></Resizable.Pane>
</Resizable.PaneGroup>
