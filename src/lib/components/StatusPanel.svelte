<script lang="ts">
  import { Ellipsis } from "lucide-svelte/icons";
  import { Button } from "$lib/components/ui/button";
  import * as Collapsible from "$lib/components/ui/collapsible";
  import * as Card from "$lib/components/ui/card";
  import type { ProcessedPage } from "$lib/types";

  let { processedPages }: { processedPages: ProcessedPage[] } = $props();

  function formatPages(pages: number[]): string {
    if (pages.length === 1) return pages[0].toString();
    if (pages.length === 2) return `${pages[0]} e ${pages[1]}`;
    return `${pages.slice(0, -1).join(", ")} e ${pages[pages.length - 1]}`;
  }
</script>

<div class="flex h-full w-full flex-col justify-between bg-accent p-2">
  <h1 class="text-xl font-semibold text-primary">Páginas processadas</h1>
  <div class="flex h-full w-full flex-col gap-2 p-2 overflow-y-auto">
    {#each processedPages as processedPage}
      {#if processedPage.pages.length > 0}
        <Card.Root>
          <Card.Header>
            <Card.Title class="flex flex-col gap-2">
              <span class="font-semibold text-primary break-all"
                >{processedPage.file_name.length > 0
                  ? "Nome sugerido"
                  : "Não foi possível sugerir um nome"} ({processedPage.pages
                  .length > 1
                  ? "páginas"
                  : "página"}: {formatPages(processedPage.pages)}):</span
              >
              <span class="break-all"
                >{processedPage.file_name ||
                  "Talvez tenha acontecido um erro, verifique as informações geradas logo abaixo"}</span
              >
            </Card.Title>
          </Card.Header>
          <Card.Content>
            <Collapsible.Root open={!processedPage.file_name}>
              <Collapsible.Trigger>
                <Button variant="secondary" size="icon"><Ellipsis /></Button>
              </Collapsible.Trigger>
              <Collapsible.Content>
                <pre
                  class="text-wrap w-full max-w-full overflow-x-auto whitespace-pre-wrap break-words">{JSON.stringify(
                    processedPage.debug,
                    null,
                    2,
                  )}</pre>
              </Collapsible.Content>
            </Collapsible.Root>
          </Card.Content>
        </Card.Root>
      {/if}
    {/each}
  </div>
</div>
