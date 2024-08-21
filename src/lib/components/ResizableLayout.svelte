<script lang="ts">
  import * as Resizable from "$lib/components/ui/resizable";
  import { createDocumentState } from "./documentContext.svelte";
  import Render from "./Render.svelte";
  import StatusPanel from "./StatusPanel.svelte";
  import { setContext } from "svelte";

  const documentContext = createDocumentState();
  setContext("documentContext", documentContext);
</script>

<Resizable.PaneGroup direction="horizontal">
  <Resizable.Pane defaultSize={30}>
    <StatusPanel />
  </Resizable.Pane>
  <Resizable.Handle />
  <Resizable.Pane>
    <Render />
  </Resizable.Pane>
</Resizable.PaneGroup>
