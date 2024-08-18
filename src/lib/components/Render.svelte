<script lang="ts">
  // FIX PROCESSING PAGES WHEN ADD OTHER DOCUMENT
  import { extractPDFImages } from '$lib/llm';
  import * as pdfjs from 'pdfjs-dist';
  import { getContext, untrack } from 'svelte';
  import { Button, buttonVariants } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as Dialog from '$lib/components/ui/dialog';
  import {
    ZoomIn,
    ZoomOut,
    ChevronFirst,
    ArrowLeft,
    ArrowRight,
    ChevronLast,
    RotateCcw,
    RotateCw,
    FolderOpen,
    FilePlus,
    FileMinus,
    FileCheck,
  } from 'lucide-svelte/icons';
  import { homeDir, resolve } from '@tauri-apps/api/path';
  import {
    readFile,
    exists,
    mkdir,
    readDir,
    remove,
  } from '@tauri-apps/plugin-fs';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { TextContent, TextItem } from 'pdfjs-dist/types/src/display/api';
  import { invoke } from '@tauri-apps/api/core';
  import type { SetupState, FileNameGeneration, ProcessingPage, ProcessedDocument } from '$lib/types';
  import { v4 as uuidv4 } from 'uuid';
  import type { DocumentState } from './documentContext.svelte';

  // Constants
  const MIN_SCALE = 0.4;
  const MAX_SCALE = 10;
  const ROTATION_INCREMENT = 90;
  const ZOOM_INCREMENT = 0.1;

  // Import PDFjs-dist worker
  pdfjs.GlobalWorkerOptions.workerSrc = new URL(
    'pdfjs-dist/build/pdf.worker.mjs',
    import.meta.url,
  ).toString();

  const documentContext: DocumentState = getContext("documentContext");

  let component: HTMLDivElement;
  let canvasContainer: HTMLDivElement;
  let renderCanvas: HTMLCanvasElement;
  let statusCanvas: HTMLCanvasElement;
  let textLayer: SVGSVGElement;

  let setup = $state<SetupState>({
    path: undefined,
    dataPath: undefined,
    document: undefined,
    page: undefined,
    scale: 1,
    rotation: 0,
    numPages: 0,
    pageNumber: 1,
    pageRendering: false,
    pageNumPending: undefined,
    metadata: undefined,
    isActive: false,
    confirmProcessDialogOpen: false,
  });

  const loadDocument = async () => {
    try {
      const data = await readFile(setup.path!);
      const document = await pdfjs.getDocument({ data }).promise;
      setup.document = document;
      setup.numPages = document.numPages;
      const metadata = await document.getMetadata();
      setup.metadata = metadata;
      console.log('Document loaded!\nMetadata:\n', metadata);
    } catch (error) {
      handleError('Error loading document:', error);
    }
  };

  const buildTextLayer = (
    viewport: pdfjs.PageViewport,
    textContent: TextContent,
  ) => {
    const svg = textLayer;
    svg.innerHTML = '';
    svg.setAttribute('width', `${viewport.width}px`);
    svg.setAttribute('height', `${viewport.height}px`);
    svg.setAttribute('font-size', '1');
    textContent.items.forEach((item) => {
      if ('str' in item) {
        const textItem = item as TextItem;
        const tx = pdfjs.Util.transform(
          pdfjs.Util.transform(viewport.transform, textItem.transform),
          [1, 0, 0, -1, 0, 0],
        );
        const style = textContent.styles[textItem.fontName];
        const text = document.createElementNS(
          'http://www.w3.org/2000/svg',
          'svg:text',
        );
        text.setAttribute('transform', `matrix(${tx.join(' ')})`);
        text.setAttribute('font-family', style.fontFamily);
        text.setAttribute('fill', 'transparent');
        text.textContent = textItem.str;
        svg.append(text);
      }
    });
  };

  const applyStatusCanvasStyles = (
    pageNumber: number,
    canvasContext: CanvasRenderingContext2D,
    viewportWidth: number,
    viewportHeight: number,
  ) => {
    const fontSize = Math.min(viewportWidth, viewportHeight) * 0.07;
    canvasContext.clearRect(0, 0, viewportWidth, viewportHeight);

    let text: string;
    let bgColor: string;
    let textColor: string;
    let borderColor: string;

    if (documentContext.processedDocuments.some(pp => pp.pages.includes(pageNumber))) {
      text = `Página ${pageNumber} processada`;
      bgColor = 'rgba(186, 79, 125, 0.2)';
      textColor = 'rgba(156, 49, 95, 1)';
      borderColor = 'rgba(186, 79, 125, 1)';
    } else if (documentContext.processingPages.some(pp => pp.pages.includes(pageNumber))) {
      text = `Página ${pageNumber} em processamento...`;
      bgColor = 'rgba(255, 223, 186, 0.5)';
      textColor = 'rgba(186, 79, 25, 1)';
      borderColor = 'rgba(255, 165, 0, 0.8)';
    } else if (documentContext.selectedPages.includes(pageNumber)) {
      text = `Página ${pageNumber} selecionada`;
      bgColor = 'rgba(173, 216, 230, 0.5)';
      textColor = 'rgba(0, 90, 156, 1)';
      borderColor = 'rgba(70, 130, 180, 0.8)';
    } else {
      return;
    }

    canvasContext.font = `bold ${fontSize}px Arial`;
    const textWidth = canvasContext.measureText(text).width;
    const x = (viewportWidth - textWidth) / 2;
    const y = (viewportHeight + fontSize) / 2 - fontSize / 2;

    canvasContext.strokeStyle = borderColor;
    canvasContext.lineWidth = 8;
    canvasContext.strokeRect(4, 4, viewportWidth - 8, viewportHeight - 8);

    canvasContext.fillStyle = bgColor;
    canvasContext.fillRect(8, 8, viewportWidth - 16, viewportHeight - 16);

    canvasContext.shadowColor = 'rgba(255, 255, 255, 0.7)';
    canvasContext.shadowBlur = 5;
    canvasContext.shadowOffsetX = 1;
    canvasContext.shadowOffsetY = 1;
    canvasContext.fillStyle = textColor;
    canvasContext.fillText(text, x, y);

    canvasContext.strokeStyle = 'rgba(0, 0, 0, 0.4)';
    canvasContext.lineWidth = 3;
    canvasContext.strokeText(text, x, y);
  };

  const loadPage = async (pageNumber: number) => {
    setup.pageRendering = true;

    try {
      const page = await setup.document!.getPage(pageNumber);
      const textContent = await page.getTextContent();

      const viewport = page.getViewport({
        scale: setup.scale,
        rotation: setup.rotation,
      });

      const { height, width } = viewport;

      renderCanvas.height = height;
      renderCanvas.width = width;
      statusCanvas.height = height;
      statusCanvas.width = width;

      const canvasContext = renderCanvas.getContext('2d');
      const statusCanvasContext = statusCanvas.getContext('2d');

      if (canvasContext && statusCanvasContext) {
        applyStatusCanvasStyles(pageNumber, statusCanvasContext, width, height);

        const renderContext = {
          canvasContext,
          viewport,
        };

        await page.render(renderContext).promise;

        buildTextLayer(viewport, textContent);
      }

      setup.pageRendering = false;

      if (setup.pageNumPending !== undefined) {
        loadPageQueue(setup.pageNumPending);
        setup.pageNumPending = undefined;
      }
    } catch (error) {
      handleError('Error loading page:', error);
      setup.pageRendering = false;
    }
  };

  const loadPageQueue = (pageNumber: number) => {
    if (setup.pageRendering) {
      setup.pageNumPending = pageNumber;
    } else {
      loadPage(pageNumber);
    }
  };

  const updatePageNumber = (delta: number) => {
    setup.pageNumber = Math.max(
      1,
      Math.min(setup.document!.numPages, setup.pageNumber + delta),
    );
  };

  const updateScale = (delta: number) => {
    const scaleFactor = Math.exp(delta);
    setup.scale = Math.min(
      MAX_SCALE,
      Math.max(MIN_SCALE, setup.scale * scaleFactor),
    );
  };

  const updateRotation = (delta: number) => {
    setup.rotation = (setup.rotation + delta + 360) % 360;
  };

  const handleFirstPage = () => (setup.pageNumber = 1);
  const handleLastPage = () => (setup.pageNumber = setup.numPages);
  const handlePrevPage = () => updatePageNumber(-1);
  const handleNextPage = () => updatePageNumber(1);
  const handleZoomIn = () => updateScale(ZOOM_INCREMENT);
  const handleZoomOut = () => updateScale(-ZOOM_INCREMENT);
  const handleRotateLeft = () => updateRotation(-ROTATION_INCREMENT);
  const handleRotateRight = () => updateRotation(ROTATION_INCREMENT);

  const handleSelectPDF = async () => {
    try {
      const file = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
        title: 'Por favor, selecione um PDF',
        defaultPath: await homeDir(),
      });
      if (file) setup.path = file.path;
    } catch (error) {
      handleError('Error selecting PDF:', error);
    }
  };

  const handleSelectPage = () => {
    if (
      documentContext.processingPages.some(pp => pp.pages.includes(validPageNumber)) ||
      documentContext.processedDocuments.some(pp => pp.pages.includes(validPageNumber))
    )
      return;
    const pageNumber = validPageNumber;
    const index = documentContext.selectedPages.indexOf(pageNumber);

    if (index !== -1) {
      documentContext.selectedPages.splice(index, 1);
    } else {
      documentContext.selectedPages.push(pageNumber);
    }
    documentContext.selectedPages.sort((a, b) => a - b);
    updatePageNumberAfterSelection(pageNumber);
    console.log(
      documentContext.selectedPages.length > 0
        ? selectedPagesText
        : 'Nenhuma página selecionada.',
    );
  };

  const updatePageNumberAfterSelection = (pageNumber: number) => {
    const currentIndex = documentContext.selectedPages.indexOf(pageNumber);
    if (currentIndex === -1) {
      const prevSelectedPage = documentContext.selectedPages
        .slice()
        .reverse()
        .find(
          (page) =>
            page < pageNumber &&
            !documentContext.processingPages.some(pp => pp.pages.includes(page)) &&
            !documentContext.processedDocuments.some(pp => pp.pages.includes(page)),
        );
      const nextSelectedPage = documentContext.selectedPages.find(
        (page) =>
          page > pageNumber &&
          !documentContext.processingPages.some(pp => pp.pages.includes(page)) &&
          !documentContext.processedDocuments.some(pp => pp.pages.includes(page)),
      );
      setup.pageNumber = prevSelectedPage || nextSelectedPage || pageNumber;
    } else {
      const prevUnselectedPage = Array.from(
        { length: setup.numPages },
        (_, i) => i + 1,
      )
        .slice()
        .reverse()
        .find(
          (page) =>
            page < pageNumber &&
            !documentContext.selectedPages.includes(page) &&
            !documentContext.processingPages.some(pp => pp.pages.includes(page)) &&
            !documentContext.processedDocuments.some(pp => pp.pages.includes(page))
        );
      const nextUnselectedPage = Array.from(
        { length: setup.numPages },
        (_, i) => i + 1,
      ).find(
        (page) =>
          page > pageNumber &&
          !documentContext.selectedPages.includes(page) &&
          !documentContext.processingPages.some(pp => pp.pages.includes(page)) &&
          !documentContext.processedDocuments.some(pp => pp.pages.includes(page))
      );
      setup.pageNumber = prevUnselectedPage || nextUnselectedPage || pageNumber;
    }
  };

  const handlePageNumberClick = (pageNumber: number) => {
    setup.pageNumber === pageNumber
      ? handleSelectPage()
      : (setup.pageNumber = pageNumber);
  };

  const handleMouseDown = (e: MouseEvent) => {
    if (e.detail > 1) e.preventDefault();
    const target = e.target as Node;
    setup.isActive =
      component.contains(target) || canvasContainer.contains(target);
  };

  const handleDoubleClick = (e: MouseEvent) =>
    setup.numPages &&
    canvasContainer.contains(e.target as Node) &&
    handleSelectPage();

  const handleWheel = (e: WheelEvent) => {
    if (!setup.numPages) return;

    setup.isActive = component.contains(e.target as Node);

    if (!setup.isActive) return;

    if (e.ctrlKey && e.shiftKey) {
      updateRotation(e.deltaY < 0 ? ROTATION_INCREMENT : -ROTATION_INCREMENT);
    } else if (e.ctrlKey) {
      updateScale(e.deltaY < 0 ? ZOOM_INCREMENT : -ZOOM_INCREMENT);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!setup.isActive) return;

    if (e.key === 'Escape') {
      e.preventDefault();
      handleSelectPDF();
      return;
    }

    if (!setup.numPages) return;

    switch (e.key) {
      case 'Home':
        handleFirstPage();
        break;
      case 'End':
        handleLastPage();
        break;
      case 'ArrowLeft':
        e.shiftKey ? handleFirstPage() : handlePrevPage();
        break;
      case 'ArrowRight':
        e.shiftKey ? handleLastPage() : handleNextPage();
        break;
      case ' ':
        e.preventDefault();
        handleSelectPage();
        break;
      case 'Backspace':
        if (documentContext.selectedPages.length) setup.pageNumber = documentContext.selectedPages.pop()!;
        break;
      case 'Enter':
        if (documentContext.selectedPages.length && !setup.confirmProcessDialogOpen) {
          e.preventDefault();
          setup.confirmProcessDialogOpen = true;
        }
        break;
      case 'Tab':
        e.preventDefault();
        if (e.shiftKey) {
          handlePrevPage();
        } else {
          handleNextPage();
        }
        break;
    }

    if (e.ctrlKey) {
      switch (e.key) {
        case '=':
          handleZoomIn();
          break;
        case '-':
          handleZoomOut();
          break;
        case 'ArrowLeft':
          handleRotateLeft();
          break;
        case 'ArrowRight':
          handleRotateRight();
          break;
      }
    }

    if (e.shiftKey) {
      switch (e.key) {
        case 'ArrowUp':
          handleZoomIn();
          break;
        case 'ArrowDown':
          handleZoomOut();
          break;
      }
    }
  };

 const handleProcessPages = () => {
    const newProcess: ProcessingPage = {
      id: uuidv4(),
      pages: [...documentContext.selectedPages],
      status: 'pending',
      startTime: Date.now(),
    };
    documentContext.processingPages = [...documentContext.processingPages, newProcess];
    documentContext.selectedPages.splice(0, documentContext.selectedPages.length);
    let processPages: string[] = [];
    for (const pageNumber of newProcess.pages) {
      processPages.push(`${setup.dataPath}\\page-${pageNumber}.webp`);
    }
    newProcess.status = 'processing';
    invoke<FileNameGeneration>('anthropic_pipeline', {
      paths: processPages,
    })
      .then((res) => {
        const newProcessedDocument: ProcessedDocument = {
          ...newProcess,
          file_name: res.file_name,
          json_file_path: res.json_file_path, // Add this line
          debug: res,
          status: 'completed',
          endTime: Date.now(),
        };
        console.log("Starting new process:", newProcessedDocument);
        documentContext.processedDocuments = [...documentContext.processedDocuments, newProcessedDocument];
        documentContext.processingPages = documentContext.processingPages.filter(pp => pp.id !== newProcess.id);
      })
      .catch((err) => {
        console.error("Error in anthropic_pipeline:", err);
        const errorProcessedDocument: ProcessedDocument = {
          ...newProcess,
          file_name: '',
          json_file_path: '', // Add this line
          debug: {} as FileNameGeneration,
          status: 'error',
          endTime: Date.now(),
          error: err.toString(),
        };
        documentContext.processedDocuments = [...documentContext.processedDocuments, errorProcessedDocument];
        documentContext.processingPages = documentContext.processingPages.filter(pp => pp.id !== newProcess.id);
      });

    console.log('Páginas em processamento: ' + newProcess.pages);
  };

  const selectedPagesText = $derived.by(() => {
    if (documentContext.selectedPages.length === 1) {
      return `Página ${documentContext.selectedPages[0]} selecionada.`;
    } else if (documentContext.selectedPages.length === 2) {
      return `Páginas ${documentContext.selectedPages[0]} e ${documentContext.selectedPages[1]} selecionadas.`;
    } else {
      return `Páginas selecionadas: ${documentContext.selectedPages.slice(0, -1).join(', ')} e ${documentContext.selectedPages.slice(-1)}.`;
    }
  });

  const validPageNumber = $derived(
    Math.min(Math.max(1, setup.pageNumber), setup.numPages),
  );

  async function checkWebpFiles(dataPath: string) {
    try {
      const files = await readDir(dataPath);
      const webpFiles = files.filter((file) => file.name.endsWith('.webp'));
      if (webpFiles.length === setup.numPages) {
        console.log('Number of .webp files matches numPages');
      } else {
        console.log(
          `Mismatch: ${webpFiles.length} .webp files found, expected ${setup.numPages}`,
        );
        await clearDataFolder(dataPath, files);
        await extractImages(dataPath);
      }
      setup.pageNumber = 1;
    } catch (error) {
      handleError('Error checking .webp files:', error);
    }
  }

  async function clearDataFolder(dataPath: string, files: any[]) {
    let allRemovalsSuccessful = true;
    for (const file of files) {
      if (file.name.endsWith('.webp')) {
        try {
          await remove(`${dataPath}/${file.name}`);
          console.log(`Removed file: ${file.name}`);
        } catch (removeError) {
          handleError(`Error removing file ${file.name}:`, removeError);
          allRemovalsSuccessful = false;
        }
      }
    }
    console.log('Data folder cleared');
    return allRemovalsSuccessful;
  }

  async function extractImages(dataPath: string) {
    try {
      const path = await resolve(dataPath, 'page-%d.webp');
      await extractPDFImages(setup.path!, path);
      console.log('Images extracted successfully');
    } catch (error) {
      handleError('Error extracting images:', error);
    }
  }

  function handleError(message: string, error: any) {
    console.error(message, error);
    // TODO: Implement user-facing error handling
  }

  // Verify data path and extract pages if necessary
  $effect.pre(() => {
    if (!setup.path || !setup.numPages) return;
    const dataPath = setup.path.endsWith('.pdf')
      ? setup.path.replace('.pdf', '-data')
      : setup.path + '-data';
    exists(dataPath)
      .then(async (isExist) => {
        if (isExist) {
          console.log('Pages data dir already exists at: ' + dataPath);
          return checkWebpFiles(dataPath);
        } else {
          console.log('Pages data not found. Extracting data from pages...');
          await mkdir(dataPath);
          return await extractImages(dataPath);
        }
      })
      .then(() => {
        setup.dataPath = dataPath;
      })
      .catch((error) => {
        handleError('Error processing PDF:', error);
      });
  });

  // Load document
  $effect(() => {
    if (!setup.path) return;
    loadDocument();
    return () => {
      setup.document?.destroy().then(() => {
        documentContext.selectedPages.splice(0, documentContext.selectedPages.length);
        setup.metadata = undefined;
        setup.numPages = 0;
        setup.pageNumber = 1;
        setup.scale = 1;
        setup.rotation = 0;
        setup.document = undefined;
      });
    };
  });

  // Load page
  $effect(() => {
    if (!setup.document || !validPageNumber) return;
    setup.scale;
    setup.rotation;
    documentContext.processingPages.length;
    documentContext.selectedPages.length;
    untrack(() => loadPageQueue(validPageNumber));
    console.log('Loading page...');
  });
</script>

<svelte:window
  onmousedown={handleMouseDown}
  ondblclick={handleDoubleClick}
  onwheel={handleWheel}
  onkeydown={handleKeyDown}
/>

<div
  bind:this={component}
  class="relative h-full w-full border-4 {setup.isActive
    ? 'border-primary'
    : 'border-accent'}"
>
  <div
    class="grid h-full w-full place-items-center overflow-auto bg-accent focus:outline-none"
  >
    <div bind:this={canvasContainer} class="relative">
      <canvas
        class={setup.numPages ? '' : 'pointer-events-none'}
        bind:this={renderCanvas}
      ></canvas>
      <svg class="absolute left-0 top-0" bind:this={textLayer}></svg>
      <canvas
        bind:this={statusCanvas}
        class="pointer-events-none absolute left-0 top-0"
      ></canvas>
    </div>
  </div>

  {#if setup.numPages && documentContext.selectedPages.length > 0}
    <div
      class="absolute left-4 top-4 flex w-1/2 select-none flex-wrap gap-1 overflow-x-auto"
    >
      {#each documentContext.selectedPages as page}
        <Button
          class="aspect-square h-8 w-8 font-semibold"
          size="sm"
          onclick={() => handlePageNumberClick(page)}
        >
          {page}
        </Button>
      {/each}
    </div>
  {/if}

  <div class="absolute right-4 top-4 flex flex-col space-y-2">
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleRotateLeft}
      disabled={!setup.numPages}
      aria-label="Rotate left"
    >
      <RotateCcw />
    </Button>
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleRotateRight}
      disabled={!setup.numPages}
      aria-label="Rotate right"
    >
      <RotateCw />
    </Button>
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleZoomIn}
      disabled={!setup.numPages || setup.scale === MAX_SCALE}
      aria-label="Zoom in"
    >
      <ZoomIn />
    </Button>
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleZoomOut}
      disabled={!setup.numPages || setup.scale === MIN_SCALE}
      aria-label="Zoom out"
    >
      <ZoomOut />
    </Button>
  </div>

  <Button
    tabindex={-1}
    class="absolute bottom-4 left-4"
    size="icon"
    onclick={handleSelectPDF}
    aria-label="Open PDF"
  >
    <FolderOpen />
  </Button>

  <div
    class="absolute bottom-4 left-1/2 flex -translate-x-1/2 scale-90 transform items-center justify-center space-x-2"
  >
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleFirstPage}
      disabled={!setup.numPages || setup.pageNumber === 1}
      aria-label="First page"
    >
      <ChevronFirst />
    </Button>
    <Button
      tabindex={-1}
      size="icon"
      onclick={handlePrevPage}
      disabled={!setup.numPages || setup.pageNumber === 1}
      aria-label="Previous page"
    >
      <ArrowLeft />
    </Button>
    <Input
      class="h-12 w-20 text-center text-2xl font-semibold text-primary focus:outline-none"
      tabindex={-1}
      type="number"
      bind:value={setup.pageNumber}
      min="1"
      max={setup.numPages}
      disabled={!setup.numPages}
      aria-label="Page number"
    />
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleNextPage}
      disabled={!setup.numPages || setup.pageNumber === setup.numPages}
      aria-label="Next page"
    >
      <ArrowRight />
    </Button>
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleLastPage}
      disabled={!setup.numPages || setup.pageNumber === setup.numPages}
      aria-label="Last page"
    >
      <ChevronLast />
    </Button>
  </div>

  <div class="absolute bottom-4 right-4 flex flex-col space-y-2">
    <Dialog.Root bind:open={setup.confirmProcessDialogOpen}>
      <Dialog.Trigger
        tabindex={-1}
        disabled={!setup.numPages ||
          documentContext.selectedPages.length === 0 ||
          documentContext.processingPages.some(pp => pp.pages.includes(setup.pageNumber))}
        class={buttonVariants({ size: 'icon', className: '' })}
        aria-label="Process selected pages"
      >
        <FileCheck />
      </Dialog.Trigger>
      <Dialog.Content class="sm:max-w-[425px]">
        <Dialog.Header>
          <Dialog.Title>
            {documentContext.selectedPages.length > 1
              ? 'Processar as páginas selecionadas?'
              : 'Processar página selecionada?'}
          </Dialog.Title>
          <Dialog.Description>
            {selectedPagesText}
          </Dialog.Description>
        </Dialog.Header>
        <Dialog.Footer>
          <Button
            onclick={() => {
              handleProcessPages();
              setup.confirmProcessDialogOpen = false;
            }}
            autofocus
          >
            Processar
          </Button>
        </Dialog.Footer>
      </Dialog.Content>
    </Dialog.Root>

    <Button
      tabindex={-1}
      size="icon"
      onclick={(e: MouseEvent) => {
        setup.isActive && e.stopImmediatePropagation();
        handleSelectPage();
      }}
      disabled={!setup.numPages ||
        documentContext.processingPages.some(pp => pp.pages.includes(setup.pageNumber)) ||
        documentContext.processedDocuments.some(pp => pp.pages.includes(setup.pageNumber))}
      aria-label={documentContext.selectedPages.includes(setup.pageNumber)
        ? 'Deselect page'
        : 'Select page'}
    >
      {#if documentContext.selectedPages.includes(setup.pageNumber)}
        <FileMinus />
      {:else}
        <FilePlus />
      {/if}
    </Button>
  </div>
</div>
