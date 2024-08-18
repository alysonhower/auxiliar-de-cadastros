<script lang="ts">
  import { Ellipsis } from "lucide-svelte/icons";
  import { Button } from "$lib/components/ui/button";
  import * as Collapsible from "$lib/components/ui/collapsible";
  import * as Card from "$lib/components/ui/card";
  import type { ProcessedDocument, ProcessingPage } from "$lib/types";

  let {
    processedDocuments,
    processingPages,
  }: {
    processedDocuments: ProcessedDocument[];
    processingPages: ProcessingPage[];
  } = $props();

  const formatPages = (pages: number[]): string => {
    if (pages.length === 1) return pages[0].toString();
    if (pages.length === 2) return `${pages[0]} e ${pages[1]}`;
    return `${pages.slice(0, -1).join(", ")} e ${pages[pages.length - 1]}`;
  };

  const formatDuration = (startTime: number, endTime?: number): string => {
    const duration = (endTime || Date.now()) - startTime;
    const seconds = Math.floor(duration / 1000);
    const minutes = Math.floor(seconds / 60);
    return minutes > 0 ? `${minutes}m ${seconds % 60}s` : `${seconds}s`;
  };

  const translateStatus = (status: string) => {
    if (status === "pending") return "pendente";
    if (status === "processing") return "processando";
    if (status === "completed") return "concluído";
    if (status === "error") return "erro";
    return "desconhecido";
  };

  let elapsedTimes = $derived.by(() => {
    let times = [];
    for (const page of processingPages) {
      page.elapsed = time.getTime() - page.startTime;
      times.push({
        id: page.id,
        elapsed: formatDuration(page.startTime),
      });
    }
    return times;
  });

  let time = $state(new Date());

  $effect(() => {
    const interval = setInterval(() => {
      time = new Date();
    }, 1000);

    return () => {
      clearInterval(interval);
    };
  });

  $effect(() => {
    processingPages.sort((a, b) => b.startTime - a.startTime);
    processedDocuments.sort((a, b) => b.endTime! - a.endTime!);
  });
</script>

<div class="flex h-full w-full flex-col justify-between bg-accent p-2">
  <h1 class="text-xl font-semibold text-primary">Status do processamento</h1>
  <div class="flex h-full w-full flex-col gap-2 p-2 overflow-y-auto">
    {#each processingPages as processingPage}
      <Card.Root>
        <Card.Header>
          <Card.Title class="flex flex-col gap-2">
            <span class="font-semibold text-primary break-all">
              Processando ({processingPage.pages.length > 1
                ? "páginas"
                : "página"}: {formatPages(processingPage.pages)})
            </span>
          </Card.Title>
        </Card.Header>
        <Card.Content>
          <p>Status: {translateStatus(processingPage.status)}</p>
          <p>
            Tempo decorrido: {#key time}<span
                >{elapsedTimes.find((t) => t.id === processingPage.id)
                  ?.elapsed}</span
              >
            {/key}
          </p>
          <p>ID: {processingPage.id}</p>
          <p>Início: {new Date(processingPage.startTime).toLocaleString()}</p>
          {#if processingPage.error}
            <p class="text-red-500">Erro: {processingPage.error}</p>
          {/if}
        </Card.Content>
      </Card.Root>
    {/each}

    {#each processedDocuments as processedDocument}
      <Card.Root>
        <Card.Header>
          <Card.Title class="flex flex-col gap-2">
            <span class="font-semibold text-primary break-all">
              {processedDocument.file_name.length > 0
                ? "Nome sugerido"
                : "Não foi possível sugerir um nome"} ({processedDocument.pages
                .length > 1
                ? "páginas"
                : "página"}: {formatPages(processedDocument.pages)}):
            </span>
            <span class="break-all">
              {processedDocument.file_name ||
                "Talvez tenha acontecido um erro, verifique as informações geradas logo abaixo"}
            </span>
          </Card.Title>
        </Card.Header>
        <Card.Content>
          <p><span class="font-semibold text-primary">Status:</span> {translateStatus(processedDocument.status)}</p>
          <p>
            <span class="font-semibold text-primary">Tempo de processamento:</span> {formatDuration(
              processedDocument.startTime,
              processedDocument.endTime,
            )}
          </p>
          <Collapsible.Root>
            <Collapsible.Trigger>
              <Button variant="secondary" size="icon"><Ellipsis /></Button>
            </Collapsible.Trigger>
            <Collapsible.Content>
              <p><span class="font-semibold text-primary">ID:</span> {processedDocument.id}</p>
              <p><span class="font-semibold text-primary">Início:</span> {new Date(processedDocument.startTime).toLocaleString()}</p>
              <p><span class="font-semibold text-primary">Fim:</span> {new Date(processedDocument.endTime!).toLocaleString()}</p>
              {#if processedDocument.error}
                <p class="text-red-500"><span class="font-semibold">Erro:</span> {processedDocument.error}</p>
              {/if}
              <p class="font-semibold text-primary">Debug:</p>
              <pre
                class="text-wrap w-full max-w-full overflow-x-auto whitespace-pre-wrap break-words">
                {JSON.stringify(processedDocument.debug, null, 2)}
              </pre>
            </Collapsible.Content>
          </Collapsible.Root>
        </Card.Content>
      </Card.Root>
    {/each}
  </div>
</div>