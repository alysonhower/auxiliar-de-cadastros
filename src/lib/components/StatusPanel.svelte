<script lang="ts">
	import { Ellipsis } from "lucide-svelte/icons";
	import { Button } from "$lib/components/ui/button";
	import * as Collapsible from "$lib/components/ui/collapsible";
	import * as Card from "$lib/components/ui/card";
	import type { FileNameGeneration, ProcessedDocument, ProcessingPage } from "$lib/types";
	import { invoke } from '@tauri-apps/api/core';
	import { Pencil, Check, X, History } from "lucide-svelte/icons";
  import { getContext } from "svelte";
  import type { DocumentState } from "./documentContext.svelte";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";

	const documentContext: DocumentState = getContext("documentContext");

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

	let editingDocumentId: string | null = $state(null);
	let editedFileName: string = $state("");

	function startEditing(document: ProcessedDocument) {
		editingDocumentId = document.id;
		editedFileName = document.file_name;
	}

  async function saveFileName(document: ProcessedDocument) {
  if (!editedFileName.trim()) {
    return;
  }

  try {
    const updatedDocumentInfo = await invoke("update_file_name", {
      path: document.json_file_path,
      name: editedFileName,
    });
    document.file_name = editedFileName;
    document.debug = updatedDocumentInfo as FileNameGeneration;
  } catch (error) {
    console.error("Error updating file name:", error);
    // Optionally, show an error message to the user
  } finally {
    editingDocumentId = null;
  }
}

	function cancelEditing() {
		editingDocumentId = null;
		editedFileName = "";
	}

	async function handleKeyDown(event: KeyboardEvent, document: ProcessedDocument) {
		if (event.key === 'Enter') {
			await saveFileName(document);
		} else if (event.key === 'Escape') {
			cancelEditing();
		}
	}

	let elapsedTimes = $derived.by(() => {
		let times = [];
		for (const page of documentContext.processingPages) {
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
		documentContext.processingPages.sort((a, b) => b.startTime - a.startTime);
		documentContext.processedDocuments.sort((a, b) => b.endTime! - a.endTime!);
	});
</script>

<div class="flex h-full w-full flex-col justify-between bg-accent p-2">
	<h1 class="text-xl font-semibold text-primary">Status do processamento</h1>
	<div class="flex h-full w-full flex-col gap-2 p-2 overflow-y-auto">
		{#each documentContext.processingPages as processingPage}
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

		{#each documentContext.processedDocuments as processedDocument}
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
						{#if editingDocumentId === processedDocument.id}
							<div class="flex items-center">
								<input
									bind:value={editedFileName}
									class="flex-grow mr-2 p-1 border rounded"
									placeholder="Enter file name"
									onkeydown={(e) => handleKeyDown(e, processedDocument)}
								/>
								<Button
									size="icon"
									variant="ghost"
									onclick={() => saveFileName(processedDocument)}
									disabled={!editedFileName.trim()}
								>
									<Check class="h-4 w-4" />
								</Button>
								<Button
									size="icon"
									variant="ghost"
									onclick={cancelEditing}
								>
									<X class="h-4 w-4" />
								</Button>
							</div>
						{:else}
							<span class="break-all flex items-center">
								{processedDocument.file_name ||
									"Talvez tenha acontecido um erro, verifique as informações geradas logo abaixo"}
								<Button
									size="icon"
									variant="ghost"
									class="ml-2"
									on:click={() => startEditing(processedDocument)}
								>
									<Pencil class="h-4 w-4" />
								</Button>
								{#if processedDocument.debug?.file_name_history && processedDocument.debug.file_name_history.length > 1}
									<DropdownMenu.Root>
										<DropdownMenu.Trigger asChild let:builder>
											<Button
												size="icon"
												variant="ghost"
												class="ml-2"
												builders={[builder]}
											>
												<History class="h-4 w-4" />
											</Button>
										</DropdownMenu.Trigger>
										<DropdownMenu.Content class="w-96 max-w-[90vw]">
											<DropdownMenu.Label>Histórico de nomes</DropdownMenu.Label>
											<DropdownMenu.Separator />
											{#each processedDocument.debug.file_name_history.slice().reverse() as historyItem, index}
												<DropdownMenu.Item
													on:click={() => {
														editedFileName = historyItem;
														saveFileName(processedDocument);
													}}
													class="break-all cursor-pointer"
												>
													{historyItem} {index === 0 ? '(atual)' : ''}
												</DropdownMenu.Item>
											{/each}
										</DropdownMenu.Content>
									</DropdownMenu.Root>
								{/if}
							</span>
						{/if}
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