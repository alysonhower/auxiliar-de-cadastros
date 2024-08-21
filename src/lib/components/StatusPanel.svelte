<script lang="ts">
  import { getContext } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    Ellipsis,
    Pencil,
    Check,
    X,
    History,
    FileCheck,
    RefreshCw,
  } from "lucide-svelte/icons";
  import { v4 as uuidv4 } from "uuid";

  import { Button, buttonVariants } from "$lib/components/ui/button";
  import * as Card from "$lib/components/ui/card";
  import * as Collapsible from "$lib/components/ui/collapsible";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import { Separator } from "$lib/components/ui/separator";
  import * as Dialog from "$lib/components/ui/dialog";

  import type {
    DocumentInfo,
    ProcessedDocument,
    ProcessingPage,
  } from "$lib/types";
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
      error: "erro",
    } as const;
    return statusMap[status as keyof typeof statusMap] || "desconhecido";
  };

  let time = $state(new Date());
  let editingDocumentId = $state<string | undefined>(undefined);
  let editedFileName = $state("");
  let showHistoryMap = $state(new Map<string, boolean>());
  let isDropdownOpenMap = $state(new Map<string, boolean>());
  let historyHoverTimeoutMap = $state(new Map<string, NodeJS.Timeout>());
  let confirmProcessDialogOpenMap = $state(new Map<string, boolean>());

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
      document.info = updatedDocumentInfo as DocumentInfo;
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

  const handleKeyDown = async (
    event: KeyboardEvent,
    document: ProcessedDocument,
  ) => {
    if (event.key === "Enter") await saveFileName(document);
    else if (event.key === "Escape") cancelEditing();
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

  const handleFinalPipeline = async (document: ProcessedDocument) => {
    if (documentContext.processingPages.some(page => page.id === document.id)) return;
    setConfirmProcessDialogOpen(document.id, false);
    documentContext.processedDocuments =
      documentContext.processedDocuments.filter(
        (doc) => doc.id !== document.id,
      );
    documentContext.processingPages = [
      ...documentContext.processingPages,
      {
        ...document,
        status: "processing",
        startTime: Date.now(),
      },
    ];

    try {
      await invoke("final_pipeline", {
        documentInfo: document.info,
      });
      documentContext.processingPages = documentContext.processingPages.filter(
        (page) => page.id !== document.id,
      );
      document.status = "completed";
      document.endTime = Date.now();
      documentContext.finishedDocuments = [
        ...documentContext.finishedDocuments,
        document,
      ];
    } catch (error) {
      console.error("Error in final pipeline:", error);
      documentContext.processingPages = documentContext.processingPages.filter(
        (page) => page.id !== document.id,
      );
      document.status = "error";
      document.error = error instanceof Error ? error.message : String(error);
      documentContext.processedDocuments = [
        ...documentContext.processedDocuments,
        document,
      ];
    }
  };

  const setConfirmProcessDialogOpen = (id: string, isOpen: boolean) => {
    confirmProcessDialogOpenMap = new Map(confirmProcessDialogOpenMap).set(id, isOpen);
  };

  const isConfirmProcessDialogOpen = (id: string) => {
    return confirmProcessDialogOpenMap.get(id) || false;
  };

  const handleRetry = async (document: ProcessedDocument) => {
    const newProcess: ProcessingPage = {
      id: uuidv4(),
      pages: [...document.pages],
      pages_paths: [...document.pages_paths],
      status: "processing",
      startTime: Date.now(),
    };

    documentContext.processingPages = [
      ...documentContext.processingPages,
      newProcess,
    ];

    documentContext.processedDocuments =
      documentContext.processedDocuments.filter(
        (doc) => doc.id !== document.id,
      );

    try {
      const res = await invoke<DocumentInfo>("anthropic_pipeline", {
        paths: newProcess.pages_paths,
      });

      const newProcessedDocument: ProcessedDocument = {
        ...newProcess,
        file_name: res.file_name,
        json_file_path: res.json_file_path,
        info: {
          ...res,
        },
        status: "completed",
        endTime: Date.now(),
      };

      documentContext.processedDocuments = [
        ...documentContext.processedDocuments,
        newProcessedDocument,
      ];
    } catch (err) {
      console.error("Error in anthropic_pipeline:", err);
      const errorProcessedDocument: ProcessedDocument = {
        ...newProcess,
        file_name: "",
        json_file_path: "",
        info: {} as DocumentInfo,
        status: "error",
        endTime: Date.now(),
        error: err instanceof Error ? err.toString() : String(err),
      };
      documentContext.processedDocuments = [
        ...documentContext.processedDocuments,
        errorProcessedDocument,
      ];
    } finally {
      documentContext.processingPages = documentContext.processingPages.filter(
        (pp) => pp.id !== newProcess.id,
      );
    }
  };

  const getMinPageNumber = (pages: number[]): number => {
    return Math.min(...pages);
  };

  const isCurrentPage = (
    document: ProcessingPage | ProcessedDocument | ProcessedDocument,
  ) => {
    return document.pages.includes(documentContext.currentPageNumber);
  };

  const handleGlobalKeydown = (event: KeyboardEvent) => {
    if (event.ctrlKey && event.key === 'Enter') {
      const currentDocument = [...documentContext.processedDocuments, ...documentContext.finishedDocuments]
        .find(doc => isCurrentPage(doc));
      if (currentDocument) {
        event.preventDefault();
        setConfirmProcessDialogOpen(currentDocument.id, true);
      }
    }
  };

  $effect(() => {
    const interval = setInterval(() => (time = new Date()), 1000);
    return () => clearInterval(interval);
  });

  $effect(() => {
    documentContext.processingPages.sort(
      (a, b) => getMinPageNumber(a.pages) - getMinPageNumber(b.pages),
    );
    documentContext.processedDocuments.sort(
      (a, b) => getMinPageNumber(a.pages) - getMinPageNumber(b.pages),
    );
    documentContext.finishedDocuments.sort(
      (a, b) => getMinPageNumber(a.pages) - getMinPageNumber(b.pages),
    );
  });

  $effect(() => {
    window.addEventListener('keydown', handleGlobalKeydown);
    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
    };
  });
</script>

<div
  class="flex h-full w-full flex-col overflow-y-auto justify-between bg-accent p-4"
>
  <h1 class="text-base font-semibold text-primary mb-4 break-all">
    Status do processamento
  </h1>
  <div class="flex h-full w-full flex-col gap-4">
    {#each documentContext.processingPages as processingPage}
      <Card.Root
        class="p-3 max-h-[50vh] flex flex-col {isCurrentPage(processingPage)
          ? 'ring-2 ring-primary'
          : ''}"
      >
        <Card.Header class="pb-1 flex-shrink-0">
          <Card.Title class="text-sm">
            <span class="font-semibold text-primary break-all">
              Processando ({processingPage.pages.length > 1
                ? "páginas"
                : "página"}: {formatPagesText(processingPage.pages)})
            </span>
          </Card.Title>
        </Card.Header>
        <Separator class="mt-1.5 bg-secondary mb-3 flex-shrink-0" />
        <Card.Content class="space-y-1 text-xs overflow-y-auto">
          <p>
            <span class="font-semibold text-primary">Status:</span>
            {translateStatusText(processingPage.status)}
          </p>
          <p>
            <span class="font-semibold text-primary">Tempo decorrido:</span>
            {#key time}<span
                >{elapsedTimes.find((t) => t.id === processingPage.id)
                  ?.elapsed}</span
              >{/key}
          </p>
          <p>
            <span class="font-semibold text-primary">Início:</span>
            {new Date(processingPage.startTime).toLocaleString()}
          </p>
          {#if processingPage.error}
            <p class="text-red-500">
              <span class="font-semibold">Erro:</span>
              {processingPage.error}
            </p>
          {/if}
        </Card.Content>
      </Card.Root>
    {/each}

    {#each documentContext.processedDocuments as processedDocument}
      <Card.Root
        class="p-3 max-h-[50vh] flex flex-col {isCurrentPage(processedDocument)
          ? 'ring-2 ring-primary'
          : ''}"
        tabindex={isCurrentPage(processedDocument) ? 0 : -1}
      >
        <Card.Header class="pb-1 flex-shrink-0">
          <Card.Title class="flex flex-col gap-1 text-sm">
            <span class="font-semibold text-primary break-all">
              Nome sugerido (página: {formatPagesText(
                processedDocument.pages,
              )}):
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
                {processedDocument.file_name ||
                  "Talvez tenha acontecido um erro, verifique as informações geradas logo abaixo"}
              </span>
              <div class="flex justify-end space-x-1 relative">
                <div class="relative">
                  {#if processedDocument.info?.file_name_history && processedDocument.info.file_name_history.length > 1}
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div
                      class="absolute right-full transition-all duration-300 ease-in-out transform"
                      class:translate-x-[100%]={!showHistoryMap.get(
                        processedDocument.id,
                      )}
                      class:opacity-0={!showHistoryMap.get(
                        processedDocument.id,
                      )}
                      class:pointer-events-none={!showHistoryMap.get(
                        processedDocument.id,
                      )}
                      onmouseenter={() =>
                        handleHistoryInteraction(processedDocument.id, true)}
                      onmouseleave={() =>
                        handleHistoryInteraction(processedDocument.id, false)}
                    >
                      <DropdownMenu.Root
                        open={isDropdownOpenMap.get(processedDocument.id)}
                      >
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
                          onmouseenter={() =>
                            handleHistoryInteraction(
                              processedDocument.id,
                              true,
                            )}
                          onmouseleave={() =>
                            handleHistoryInteraction(
                              processedDocument.id,
                              false,
                            )}
                        >
                          <div
                            class="sticky top-0 bg-background z-10 py-1.5 px-2"
                          >
                            <DropdownMenu.Label
                              >Histórico de nomes</DropdownMenu.Label
                            >
                            <DropdownMenu.Separator />
                          </div>
                          <div class="overflow-y-auto">
                            {#each processedDocument.info.file_name_history.slice() as historyItem}
                              <DropdownMenu.Item
                                onclick={() => {
                                  editedFileName = historyItem;
                                  saveFileName(processedDocument);
                                  isDropdownOpenMap = setMapValue(
                                    isDropdownOpenMap,
                                    processedDocument.id,
                                    false,
                                  );
                                }}
                                class="break-all cursor-pointer"
                              >
                                {historyItem}
                                {historyItem === processedDocument.file_name
                                  ? "(nome atual)"
                                  : ""}
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
                    onmouseenter={() =>
                      handleEditButtonInteraction(processedDocument.id, true)}
                    onmouseleave={() =>
                      handleEditButtonInteraction(processedDocument.id, false)}
                  >
                    <Pencil class="h-3.5 w-3.5" />
                  </Button>

                  <Dialog.Root
                    open={isConfirmProcessDialogOpen(processedDocument.id)}
                    onOpenChange={(open) =>
                      setConfirmProcessDialogOpen(processedDocument.id, open)}
                  >
                    <Dialog.Trigger
                      tabindex={-1}
                      disabled={showHistoryMap.get(processedDocument.id) ||
                        editingDocumentId === processedDocument.id}
                      class={buttonVariants({
                        size: "icon",
                        className: "h-7 w-7",
                      })}
                      aria-label="Process selected pages"
                    >
                      <FileCheck class="h-3.5 w-3.5" />
                    </Dialog.Trigger>
                    <Dialog.Content class="sm:max-w-[425px]">
                      <Dialog.Header>
                        <Dialog.Title>
                          {processedDocument.pages.length > 1
                            ? "Processar as páginas selecionadas?"
                            : "Processar página selecionada?"}
                        </Dialog.Title>
                        <Dialog.Description>
                          Você está prestes a processar {processedDocument.pages
                            .length > 1
                            ? "as páginas"
                            : "a página"}
                          {formatPagesText(processedDocument.pages)}. Deseja
                          continuar?
                        </Dialog.Description>
                      </Dialog.Header>
                      <Dialog.Footer>
                        <Button
                          onclick={() => handleFinalPipeline(processedDocument)}
                        >
                          Processar
                        </Button>
                      </Dialog.Footer>
                    </Dialog.Content>
                  </Dialog.Root>
                </div>
              </div>
            {/if}
          </Card.Title>
        </Card.Header>
        <Separator class="mt-1.5 bg-secondary mb-3 flex-shrink-0" />
        <Card.Content class="space-y-1 text-xs overflow-y-auto">
          <p>
            <span class="font-semibold text-primary">Status:</span>
            {translateStatusText(processedDocument.status)}
          </p>
          <p>
            <span class="font-semibold text-primary"
              >Tempo de processamento:</span
            >
            {formatDurationText(
              processedDocument.startTime,
              processedDocument.endTime,
            )}
          </p>
          {#if processedDocument.status === "error"}
            <p class="text-red-500">
              <span class="font-semibold">Erro:</span>
              {processedDocument.error}
            </p>
            <Button
              size="sm"
              variant="outline"
              onclick={() => handleRetry(processedDocument)}
              class="mt-2"
            >
              <RefreshCw class="h-3.5 w-3.5 mr-1" />
              Tentar novamente
            </Button>
          {/if}
          <Collapsible.Root>
            <div class="sticky top-0">
              <Collapsible.Trigger>
                <Button variant="secondary" size="icon" class="h-6 w-6 mt-1">
                  <Ellipsis class="h-3 w-3" />
                </Button>
              </Collapsible.Trigger>
            </div>
            <Collapsible.Content class="mt-1.5 space-y-1">
              <p>
                <span class="font-semibold text-primary">ID:</span>
                {processedDocument.id}
              </p>
              <p>
                <span class="font-semibold text-primary">Início:</span>
                {new Date(processedDocument.startTime).toLocaleString()}
              </p>
              <p>
                <span class="font-semibold text-primary">Fim:</span>
                {new Date(processedDocument.endTime!).toLocaleString()}
              </p>
              {#if processedDocument.error}
                <p class="text-red-500">
                  <span class="font-semibold">Erro:</span>
                  {processedDocument.error}
                </p>
              {/if}
              <p class="font-semibold text-primary">info:</p>
              <pre
                class="text-wrap w-full max-w-full overflow-x-auto whitespace-pre-wrap break-words text-[10px]">
                {JSON.stringify(processedDocument.info, null, 2)}
              </pre>
            </Collapsible.Content>
          </Collapsible.Root>
        </Card.Content>
      </Card.Root>
    {/each}

    {#each documentContext.finishedDocuments as finishedDocument}
      <Card.Root
        class="p-3 max-h-[50vh] flex flex-col {isCurrentPage(finishedDocument)
          ? 'ring-2 ring-primary'
          : ''}"
        tabindex={isCurrentPage(finishedDocument) ? 0 : -1}
      >
        <Card.Header class="pb-1 flex-shrink-0">
          <Card.Title class="flex flex-col gap-1 text-sm">
            <span class="font-semibold text-primary break-all">
              Documento Finalizado:
            </span>
            <span class="break-all w-full font-semibold text-sm mb-1">
              {finishedDocument.file_name}
            </span>
          </Card.Title>
        </Card.Header>
        <Separator class="mt-1.5 bg-secondary mb-3 flex-shrink-0" />
        <Card.Content class="space-y-1 text-xs overflow-y-auto">
          <p>
            <span class="font-semibold text-primary">Status:</span>
            Finalizado
          </p>
          <p>
            <span class="font-semibold text-primary">Páginas:</span>
            {formatPagesText(finishedDocument.pages)}
          </p>
          <p>
            <span class="font-semibold text-primary"
              >Tempo de processamento:</span
            >
            {formatDurationText(
              finishedDocument.startTime,
              finishedDocument.endTime,
            )}
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
              <p>
                <span class="font-semibold text-primary">ID:</span>
                {finishedDocument.id}
              </p>
              <p>
                <span class="font-semibold text-primary">Início:</span>
                {new Date(finishedDocument.startTime).toLocaleString()}
              </p>
              <p>
                <span class="font-semibold text-primary">Fim:</span>
                {new Date(finishedDocument.endTime!).toLocaleString()}
              </p>
              <p class="font-semibold text-primary">info:</p>
              <pre
                class="text-wrap w-full max-w-full overflow-x-auto whitespace-pre-wrap break-words text-[10px]">
                {JSON.stringify(finishedDocument.info, null, 2)}
              </pre>
            </Collapsible.Content>
          </Collapsible.Root>
        </Card.Content>
      </Card.Root>
    {/each}
  </div>
</div>
