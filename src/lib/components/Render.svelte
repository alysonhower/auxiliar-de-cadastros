<script lang="ts">
  import { extractPDFImages } from '$lib/llm';

  import * as pdfjs from 'pdfjs-dist';

  import { untrack } from 'svelte';

  import { Button, buttonVariants } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as Dialog from '$lib/components/ui/dialog';

  import ZoomIn from 'lucide-svelte/icons/zoom-in';
  import ZoomOut from 'lucide-svelte/icons/zoom-out';
  import ChevronFirst from 'lucide-svelte/icons/chevron-first';
  import ArrowLeft from 'lucide-svelte/icons/arrow-left';
  import ArrowRight from 'lucide-svelte/icons/arrow-right';
  import ChevronLast from 'lucide-svelte/icons/chevron-last';
  import RotateCcw from 'lucide-svelte/icons/rotate-ccw';
  import RotateCw from 'lucide-svelte/icons/rotate-cw';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import FilePlus from 'lucide-svelte/icons/file-plus';
  import FileMinus from 'lucide-svelte/icons/file-minus';
  import FileCheck from 'lucide-svelte/icons/file-check';

  import { homeDir, resolve } from '@tauri-apps/api/path';
  import {
    readFile,
    exists,
    mkdir,
    readDir,
    remove,
  } from '@tauri-apps/plugin-fs';
  import { open } from '@tauri-apps/plugin-dialog';

  import type {
    PDFDocumentProxy,
    PDFPageProxy,
  } from 'pdfjs-dist/types/src/pdf.d.ts';

  import type { TextContent, TextItem } from 'pdfjs-dist/types/src/display/api';

  pdfjs.GlobalWorkerOptions.workerSrc = new URL(
    'pdfjs-dist/build/pdf.worker.mjs',
    import.meta.url,
  ).toString();

  let {
    selectedPages,
    processingPages,
  }: { selectedPages: number[]; processingPages: number[] } = $props();

  let component: HTMLDivElement;
  let canvasContainer: HTMLDivElement;
  let renderCanvas: HTMLCanvasElement;
  let statusCanvas: HTMLCanvasElement;
  let textLayer: SVGSVGElement;

  let setup = $state({
    path: undefined as string | undefined,
    dataPath: undefined as string | undefined,
    document: undefined as PDFDocumentProxy | undefined,
    page: undefined as PDFPageProxy | undefined,
    scale: 1,
    rotation: 0,
    numPages: 0,
    pageNumber: 1,
    pageRendering: false,
    pageNumPending: undefined as number | undefined,
    metadata: undefined as undefined | any,
    isActive: false,
    confirmProcessDialogOpen: false,
  });

  const loadDocument = async () => {
    try {
      const data = await readFile(setup.path!);
      const document = await pdfjs.getDocument({ data }).promise;
      setup.document = document;
      setup.numPages = document.numPages;
      document.getMetadata().then((metadata) => {
        setup.metadata = metadata;
        console.log('Document loaded!\nMetadata:\n', metadata);
      });
    } catch (error) {
      console.error('Error loading document:', error);
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
    const fontSize = Math.min(viewportWidth, viewportHeight) * 0.05;
    if (processingPages.includes(pageNumber)) {
      canvasContext.clearRect(0, 0, viewportWidth, viewportHeight);
      const text = `Página ${pageNumber} em processamento...`;
      canvasContext.font = `bold ${fontSize}px Arial`;
      const textWidth = canvasContext.measureText(text).width;
      const x = (viewportWidth - textWidth) / 2;
      const y = (viewportHeight + fontSize) / 2 - fontSize / 2;
      canvasContext.fillStyle = 'rgba(186, 79, 125, 0.7)';
      canvasContext.fillRect(0, 0, viewportWidth, viewportHeight);
      canvasContext.fillStyle = 'rgba(232, 227, 229, 1)';
      canvasContext.fillText(text, x, y);
    } else if (selectedPages.includes(pageNumber)) {
      canvasContext.clearRect(0, 0, viewportWidth, viewportHeight);
      const text = `Página ${pageNumber} selecionada`;
      canvasContext.font = `bold ${fontSize}px Arial`;
      const textWidth = canvasContext.measureText(text).width;
      const x = (viewportWidth - textWidth) / 2;
      const y = (viewportHeight + fontSize) / 2 - fontSize / 2;
      canvasContext.fillStyle = 'rgba(232, 227, 229, 0.7)';
      canvasContext.fillRect(0, 0, viewportWidth, viewportHeight);
      canvasContext.fillStyle = 'rgba(186, 79, 125, 1)';
      canvasContext.fillText(text, x, y);
    } else {
      canvasContext.clearRect(0, 0, viewportWidth, viewportHeight);
    }
  };

  const loadPage = async (pageNumber: number) => {
    setup.pageRendering = true;

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

      const renderTask = page.render(renderContext);
      await renderTask.promise;

      buildTextLayer(viewport, textContent);
    }

    setup.pageRendering = false;

    if (setup.pageNumPending !== undefined) {
      loadPageQueue(setup.pageNumPending);
      setup.pageNumPending = undefined;
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
    setup.scale = Math.min(10, Math.max(0.4, setup.scale! * scaleFactor));
  };

  const updateRotation = (delta: number) => {
    setup.rotation = (setup.rotation + delta + 360) % 360;
  };

  const handleFirstPage = () => (setup.pageNumber = 1);
  const handleLastPage = () => (setup.pageNumber = setup.numPages!);
  const handlePrevPage = () => updatePageNumber(-1);
  const handleNextPage = () => updatePageNumber(1);
  const handleZoomIn = () => updateScale(0.4);
  const handleZoomOut = () => updateScale(-0.4);
  const handleRotateLeft = () => updateRotation(-90);
  const handleRotateRight = () => updateRotation(90);

  const handleSelectPDF = async () => {
    const file = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
      title: 'Por favor, selecione um PDF',
      defaultPath: await homeDir(),
    });
    if (file) setup.path = file.path;
  };

  const handleSelectPage = () => {
    if (processingPages.includes(validPageNumber)) return;
    const pageNumber = validPageNumber;
    const index = selectedPages.indexOf(pageNumber);
    if (index !== -1) {
      selectedPages.splice(index, 1);
    } else {
      selectedPages.push(pageNumber);
    }
    selectedPages.sort((a, b) => a - b);
    const currentIndex = selectedPages.indexOf(pageNumber);
    if (currentIndex === -1) {
      const prevSelectedPage = selectedPages
        .slice()
        .reverse()
        .find((page) => page < pageNumber && !processingPages.includes(page));
      const nextSelectedPage = selectedPages.find(
        (page) => page > pageNumber && !processingPages.includes(page),
      );
      setup.pageNumber = prevSelectedPage || nextSelectedPage || pageNumber;
    } else {
      const prevUnselectedPage = Array.from(
        { length: setup.numPages! },
        (_, i) => i + 1,
      )
        .slice()
        .reverse()
        .find(
          (page) =>
            page < pageNumber &&
            !selectedPages.includes(page) &&
            !processingPages.includes(page),
        );
      const nextUnselectedPage = Array.from(
        { length: setup.numPages! },
        (_, i) => i + 1,
      ).find(
        (page) =>
          page > pageNumber &&
          !selectedPages.includes(page) &&
          !processingPages.includes(page),
      );
      setup.pageNumber = prevUnselectedPage || nextUnselectedPage || pageNumber;
    }
    console.log(
      selectedPages.length > 0
        ? selectedPagesText
        : 'Nenhuma página selecionada.',
    );
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
      updateRotation(e.deltaY < 0 ? 90 : -90);
    } else if (e.ctrlKey) {
      updateScale(e.deltaY < 0 ? 0.1 : -0.1);
    } else if (e.altKey) {
      updatePageNumber(e.deltaY < 0 ? -1 : 1);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!setup.isActive) return;

    e.key === 'Escape' && handleSelectPDF();

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
        if (selectedPages.length) setup.pageNumber = selectedPages.pop()!;
        break;
      case 'Enter':
        if (selectedPages.length && !setup.confirmProcessDialogOpen)
          setup.confirmProcessDialogOpen = true;
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
    processingPages.push(...selectedPages);
    selectedPages.splice(0, selectedPages.length);
    console.log('Páginas processadas: ' + processingPages);
  };

  const selectedPagesText = $derived.by(() => {
    if (selectedPages.length === 1) {
      return `Página ${selectedPages[0]} selecionada.`;
    } else if (selectedPages.length === 2) {
      return `Páginas ${selectedPages[0]} e ${selectedPages[1]} selecionadas.`;
    } else {
      return `Páginas selecionadas: ${selectedPages.slice(0, -1).join(', ')} e ${selectedPages.slice(-1)}.`;
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
        let allRemovalsSuccessful = true;
        for (const file of files) {
          if (file.name.endsWith('.webp')) {
            try {
              await remove(`${dataPath}/${file.name}`);
              console.log(`Removed file: ${file.name}`);
            } catch (removeError) {
              console.error(`Error removing file ${file.name}:`, removeError);
              allRemovalsSuccessful = false;
            }
          }
        }
        console.log('Data folder cleared');
        if (allRemovalsSuccessful) {
          await resolve(dataPath, 'page-%d.webp').then((path) => {
            extractPDFImages(setup.path!, path).then(() =>
              console.log('Images extracted successfully'),
            );
          });
        }
      }
      setup.pageNumber = 1;
    } catch (error) {
      console.error('Error checking .webp files:', error);
    }
  }

  // Verify data path and extract pages if necessary
  $effect(() => {
    if (!setup.path || !setup.numPages) return;
    const dataPath = setup.path.endsWith('.pdf')
      ? setup.path.replace('.pdf', '-data')
      : setup.path + '-data';
    exists(dataPath)
      .then((isExist) => {
        if (isExist) {
          console.log('Pages data dir already exists at: ' + dataPath);
          checkWebpFiles(dataPath).then(() => {
            console.log('Number of .webp files matches numPages');
          });
        } else {
          console.log('Pages data not found. Extracting data from pages...');
          mkdir(dataPath).then(() => {
            resolve(dataPath, 'page-%d.webp').then((path) => {
              extractPDFImages(setup.path!, path).then(() =>
                console.log('Images extracted successfully'),
              );
            });
          });
        }
      })
      .then(() => (setup.dataPath = dataPath));
  });

  // Load document
  $effect(() => {
    if (!setup.path) return;
    loadDocument();
    return () => {
      setup.document?.destroy().then(() => {
        selectedPages.splice(0, selectedPages.length);
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
    processingPages.length;
    selectedPages.length;
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

  {#if setup.numPages && selectedPages.length > 0}
    <div
      class="absolute left-4 top-4 flex w-1/2 select-none flex-wrap gap-1 overflow-x-auto"
    >
      {#each selectedPages as page}
        <Button
          class=" aspect-square h-8 w-8 font-semibold"
          size="sm"
          onclick={() => handlePageNumberClick(page)}>{page}</Button
        >
      {/each}
    </div>
  {/if}

  <div class="absolute right-4 top-4 flex flex-col space-y-2">
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleRotateLeft}
      disabled={!setup.numPages}><RotateCcw /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleRotateRight}
      disabled={!setup.numPages}><RotateCw /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleZoomIn}
      disabled={!setup.numPages || setup.scale === 10}><ZoomIn /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleZoomOut}
      disabled={!setup.numPages || setup.scale === 0.5}><ZoomOut /></Button
    >
  </div>

  <Button
    tabindex={-1}
    class="absolute bottom-4 left-4 "
    size="icon"
    onclick={handleSelectPDF}><FolderOpen /></Button
  >

  <div
    class="absolute bottom-4 left-1/2 flex -translate-x-1/2 scale-90 transform items-center justify-center space-x-2"
  >
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleFirstPage}
      disabled={!setup.numPages || setup.pageNumber === 1}
      ><ChevronFirst /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      onclick={handlePrevPage}
      disabled={!setup.numPages || setup.pageNumber === 1}><ArrowLeft /></Button
    >
    <Input
      class="h-12 w-20 text-center text-2xl font-semibold text-primary focus:outline-none"
      tabindex={-1}
      type="number"
      bind:value={setup.pageNumber}
      min="1"
      max={setup.numPages}
      disabled={!setup.numPages}
    />
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleNextPage}
      disabled={!setup.numPages || setup.pageNumber === setup.numPages}
      ><ArrowRight /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      onclick={handleLastPage}
      disabled={!setup.numPages || setup.pageNumber === setup.numPages}
      ><ChevronLast /></Button
    >
  </div>

  <div class="absolute bottom-4 right-4 flex flex-col space-y-2">
    <Dialog.Root bind:open={setup.confirmProcessDialogOpen}>
      <Dialog.Trigger
        tabindex={-1}
        disabled={!setup.numPages ||
          selectedPages.length === 0 ||
          processingPages.includes(setup.pageNumber)}
        class={buttonVariants({ size: 'icon', className: '' })}
      >
        <FileCheck />
      </Dialog.Trigger>
      <Dialog.Content class="sm:max-w-[425px]">
        <Dialog.Header>
          <Dialog.Title>
            {selectedPages.length > 1
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
            }}>Processar</Button
          >
        </Dialog.Footer>
      </Dialog.Content>
    </Dialog.Root>

    <Button
      tabindex={-1}
      size="icon"
      onclick={(e: MouseEvent) => (
        setup.isActive && e.stopImmediatePropagation(), handleSelectPage()
      )}
      disabled={!setup.numPages || processingPages.includes(setup.pageNumber)}
    >
      {#if selectedPages.includes(setup.pageNumber)}
        <FileMinus />
      {:else}
        <FilePlus />
      {/if}
    </Button>
  </div>
</div>
