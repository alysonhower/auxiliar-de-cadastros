<script lang="ts">
import { getContext } from "svelte";
import { invoke } from '@tauri-apps/api/core';
import { Ellipsis, Pencil, Check, X, History } from "lucide-svelte/icons";

import { Button } from "$lib/components/ui/button";
import * as Card from "$lib/components/ui/card";
import * as Collapsible from "$lib/components/ui/collapsible";
import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
import { Separator } from "$lib/components/ui/separator";

import type { FileNameGeneration, ProcessedDocument } from "$lib/types";
import type { DocumentState } from "./documentContext.svelte";

const documentContext: DocumentState = getContext("documentContext");

const formatPagesText = (pages: number[]): string => {
  if (pages.length === 1) return pages[0].toString();
  if (pages.length === 2) return `${pages[0]} e ${pages[1]}`;
  return `${pages.slice(0, -1).join(", ")} e ${pages[pages.length - 1]}`;
};

const formatDurationText = (startTime: number, endTime?: number): string => {
  const duration = (endTime || Date.now()) - startTime;
  const seconds = Math.floor(duration / 1000);
  const minutes = Math.floor(seconds / 60);
  return minutes > 0 ? `${minutes}m ${seconds % 60}s` : `${seconds}s`;
};

const translateStatusText = (status: string): string => {
  const statusMap = {
    pending: "pendente",
    processing: "processando",
    completed: "concluído",
    error: "erro"
  } as const;
  return statusMap[status as keyof typeof statusMap] || "desconhecido";
};

let time = $state(new Date());
let editingDocumentId = $state<string | undefined>(undefined);
let editedFileName = $state("");
let showHistoryMap = $state(new Map<string, boolean>());
let isDropdownOpenMap = $state(new Map<string, boolean>());
let historyHoverTimeoutMap = $state(new Map<string, NodeJS.Timeout>());

const setMapValue = (map: Map<string, any>, id: string, value: any) => {
  return new Map(map).set(id, value);
};

const handleHistoryInteraction = (id: string, isEnter: boolean) => {
  clearTimeout(historyHoverTimeoutMap.get(id));
  if (isEnter) {
    showHistoryMap = setMapValue(showHistoryMap, id, true);
    isDropdownOpenMap = setMapValue(isDropdownOpenMap, id, true);
  } else {
    const timeout = setTimeout(() => {
      showHistoryMap = setMapValue(showHistoryMap, id, false);
      isDropdownOpenMap = setMapValue(isDropdownOpenMap, id, false);
    }, 300);
    historyHoverTimeoutMap = setMapValue(historyHoverTimeoutMap, id, timeout);
  }
};

const handleEditButtonInteraction = (id: string, isEnter: boolean) => {
  if (isEnter) {
    showHistoryMap = setMapValue(showHistoryMap, id, true);
  } else {
    const timeout = setTimeout(() => {
      if (!isDropdownOpenMap.get(id)) {
        showHistoryMap = setMapValue(showHistoryMap, id, false);
      }
    }, 300);
    historyHoverTimeoutMap = setMapValue(historyHoverTimeoutMap, id, timeout);
  }
};

const startEditing = (document: ProcessedDocument) => {
  editingDocumentId = document.id;
  editedFileName = document.file_name;
};

const saveFileName = async (document: ProcessedDocument) => {
  if (!editedFileName.trim()) return;

  try {
    const updatedDocumentInfo = await invoke("update_file_name", {
      path: document.json_file_path,
      name: editedFileName,
    });
    document.file_name = editedFileName;
    document.debug = updatedDocumentInfo as FileNameGeneration;
    showHistoryMap = setMapValue(showHistoryMap, document.id, false);
  } catch (error) {
    console.error("Error updating file name:", error);
  } finally {
    editingDocumentId = undefined;
  }
};

const cancelEditing = () => {
  editingDocumentId = undefined;
  editedFileName = "";
};

const handleKeyDown = async (event: KeyboardEvent, document: ProcessedDocument) => {
  if (event.key === 'Enter') await saveFileName(document);
  else if (event.key === 'Escape') cancelEditing();
};

const elapsedTimes = $derived.by(() => {
  let times = [];
  for (const page of documentContext.processingPages) {
    page.elapsed = time.getTime() - page.startTime;
    times.push({
      id: page.id,
      elapsed: formatDurationText(page.startTime),
    });
  }
  return times;
});


$effect(() => {
  const interval = setInterval(() => time = new Date(), 1000);
  return () => clearInterval(interval);
});

$effect(() => {
  documentContext.processingPages.sort((a, b) => b.startTime - a.startTime);
  documentContext.processedDocuments.sort((a, b) => b.endTime! - a.endTime!);
});
</script>

<div class="flex h-full w-full flex-col overflow-y-auto justify-between bg-accent p-4">
  <h1 class="text-base font-semibold text-primary mb-4 break-all">Status do processamento</h1>
  <div class="flex h-full w-full flex-col gap-4">
    {#each documentContext.processingPages as processingPage}
      <Card.Root class="p-3 max-h-[50vh] flex flex-col">
        <Card.Header class="pb-1 flex-shrink-0">
          <Card.Title class="text-sm">
            <span class="font-semibold text-primary break-all">
              Processando ({processingPage.pages.length > 1 ? "páginas" : "página"}: {formatPagesText(processingPage.pages)})
            </span>
          </Card.Title>
        </Card.Header>
        <Separator class="mt-1.5 bg-secondary mb-3 flex-shrink-0" />
        <Card.Content class="space-y-1 text-xs overflow-y-auto">
          <p><span class="font-semibold text-primary">Status:</span> {translateStatusText(processingPage.status)}</p>
          <p>
            <span class="font-semibold text-primary">Tempo decorrido:</span>
            {#key time}<span>{elapsedTimes.find((t) => t.id === processingPage.id)?.elapsed}</span>{/key}
          </p>
          <p><span class="font-semibold text-primary">Início:</span> {new Date(processingPage.startTime).toLocaleString()}</p>
          {#if processingPage.error}
            <p class="text-red-500"><span class="font-semibold">Erro:</span> {processingPage.error}</p>
          {/if}
        </Card.Content>
      </Card.Root>
    {/each}

    {#each documentContext.processedDocuments as processedDocument}
      <Card.Root class="p-3 max-h-[50vh] flex flex-col">
        <Card.Header class="pb-1 flex-shrink-0">
          <Card.Title class="flex flex-col gap-1 text-sm">
            <span class="font-semibold text-primary break-all">
              Nome sugerido (página: {formatPagesText(processedDocument.pages)}):
            </span>
            {#if editingDocumentId === processedDocument.id}
              <div class="w-full h-full space-y-1">
                <!-- svelte-ignore a11y_autofocus -->
                <div
                  class="w-full p-2 border rounded-md font-semibold text-sm focus:outline-none focus:ring-1 focus:ring-primary break-all min-h-[1.5em] max-h-[50vh] overflow-y-auto"
                  contenteditable="true"
                  bind:textContent={editedFileName}
                  onkeydown={(e) => handleKeyDown(e, processedDocument)}
                  onfocus={(e) => {
                    if (e.target instanceof Node) {
                      const range = document.createRange();
                      range.selectNodeContents(e.target);
                      const selection = window.getSelection();
                      selection?.removeAllRanges();
                      selection?.addRange(range);
                    }
                  }}
                  role="textbox"
                  tabindex="0"
                  autofocus
                ></div>
                <div class="flex justify-end space-x-1">
                  <Button
                    size="icon"
                    variant="default"
                    onclick={() => saveFileName(processedDocument)}
                    disabled={!editedFileName.trim()}
                    class="h-7 w-7"
                  >
                    <Check class="h-3.5 w-3.5" />
                  </Button>
                  <Button
                    size="icon"
                    variant="default"
                    onclick={cancelEditing}
                    class="h-7 w-7"
                  >
                    <X class="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            {:else}
              <span class="break-all w-full font-semibold text-sm mb-1">
                {processedDocument.file_name || "Talvez tenha acontecido um erro, verifique as informações geradas logo abaixo"}
              </span>
              <div class="flex justify-end space-x-1 relative">
                <div class="relative">
                  {#if processedDocument.debug?.file_name_history && processedDocument.debug.file_name_history.length > 1}
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div
                      class="absolute right-full transition-all duration-300 ease-in-out transform"
                      class:translate-x-[100%]={!showHistoryMap.get(processedDocument.id)}
                      class:opacity-0={!showHistoryMap.get(processedDocument.id)}
                      class:pointer-events-none={!showHistoryMap.get(processedDocument.id)}
                      onmouseenter={() => handleHistoryInteraction(processedDocument.id, true)}
                      onmouseleave={() => handleHistoryInteraction(processedDocument.id, false)}
                    >
                      <DropdownMenu.Root open={isDropdownOpenMap.get(processedDocument.id)}>
                        <DropdownMenu.Trigger asChild let:builder>
                          <Button
                            size="icon"
                            variant="default"
                            builders={[builder]}
                            class="h-7 w-7 mr-1"
                          >
                            <History class="h-3.5 w-3.5" />
                          </Button>
                        </DropdownMenu.Trigger>
                        <DropdownMenu.Content
                          class="w-96 max-w-[90vw] max-h-60 overflow-hidden flex flex-col"
                          onmouseenter={() => handleHistoryInteraction(processedDocument.id, true)}
                          onmouseleave={() => handleHistoryInteraction(processedDocument.id, false)}
                        >
                          <div class="sticky top-0 bg-background z-10 py-1.5 px-2">
                            <DropdownMenu.Label>Histórico de nomes</DropdownMenu.Label>
                            <DropdownMenu.Separator />
                          </div>
                          <div class="overflow-y-auto">
                            {#each processedDocument.debug.file_name_history.slice() as historyItem}
                              <DropdownMenu.Item
                                onclick={() => {
                                  editedFileName = historyItem;
                                  saveFileName(processedDocument);
                                  isDropdownOpenMap = setMapValue(isDropdownOpenMap, processedDocument.id, false);
                                }}
                                class="break-all cursor-pointer"
                              >
                                {historyItem} {historyItem === processedDocument.file_name ? '(nome atual)' : ''}
                              </DropdownMenu.Item>
                            {/each}
                          </div>
                        </DropdownMenu.Content>
                      </DropdownMenu.Root>
                    </div>
                  {/if}
                  <Button
                    size="icon"
                    variant="default"
                    onclick={() => startEditing(processedDocument)}
                    class="h-7 w-7 relative z-10"
                    onmouseenter={() => handleEditButtonInteraction(processedDocument.id, true)}
                    onmouseleave={() => handleEditButtonInteraction(processedDocument.id, false)}
                  >
                    <Pencil class="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            {/if}
          </Card.Title>
        </Card.Header>
        <Separator class="mt-1.5 bg-secondary mb-3 flex-shrink-0" />
        <Card.Content class="space-y-1 text-xs overflow-y-auto">
          <p><span class="font-semibold text-primary">Status:</span> {translateStatusText(processedDocument.status)}</p>
          <p>
            <span class="font-semibold text-primary">Tempo de processamento:</span>
            {formatDurationText(processedDocument.startTime, processedDocument.endTime)}
          </p>
          <Collapsible.Root>
            <div class="sticky top-0">
              <Collapsible.Trigger>
                <Button variant="secondary" size="icon" class="h-6 w-6 mt-1">
                  <Ellipsis class="h-3 w-3" />
                </Button>
              </Collapsible.Trigger>
            </div>
            <Collapsible.Content class="mt-1.5 space-y-1">
              <p><span class="font-semibold text-primary">ID:</span> {processedDocument.id}</p>
              <p><span class="font-semibold text-primary">Início:</span> {new Date(processedDocument.startTime).toLocaleString()}</p>
              <p><span class="font-semibold text-primary">Fim:</span> {new Date(processedDocument.endTime!).toLocaleString()}</p>
              {#if processedDocument.error}
                <p class="text-red-500"><span class="font-semibold">Erro:</span> {processedDocument.error}</p>
              {/if}
              <p class="font-semibold text-primary">Debug:</p>
              <pre class="text-wrap w-full max-w-full overflow-x-auto whitespace-pre-wrap break-words text-[10px]">
                {JSON.stringify(processedDocument.debug, null, 2)}
              </pre>
            </Collapsible.Content>
          </Collapsible.Root>
        </Card.Content>
      </Card.Root>
    {/each}
  </div>
</div>