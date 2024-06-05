<script lang="ts">
  import * as pdfjs from 'pdfjs-dist';
  import { homeDir } from '@tauri-apps/api/path';
  import { readFile } from '@tauri-apps/plugin-fs';
  import { open } from '@tauri-apps/plugin-dialog';
  import { tick, untrack } from 'svelte';

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
  let selectedCanvas: HTMLCanvasElement;
  let processingCanvas: HTMLCanvasElement;
  let textLayerSvg: SVGSVGElement;

  let setup = $state({
    path: undefined as string | undefined,
    scale: undefined as number | undefined,
    rotation: 0,
    numPages: undefined as number | undefined,
    pageNumber: 1,
    pageRendering: false as boolean,
    pageNumPending: undefined as number | undefined,
    document: undefined as PDFDocumentProxy | undefined,
    page: undefined as PDFPageProxy | undefined,
    isActive: false,
    confirmProcessDialogOpen: false,
  });

  const loadDocument = async () => {
    const data = await readFile(setup.path!);
    return await pdfjs.getDocument({ data }).promise;
  };

  const buildTextLayer = (
    viewport: pdfjs.PageViewport,
    textContent: TextContent,
  ) => {
    const svg = textLayerSvg;
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

  const loadPage = async (pageNumber: number) => {
    setup.pageRendering = true;

    const page = await setup.document!.getPage(pageNumber);
    const textContent = await page.getTextContent();

    if (!setup.scale) {
      setup.scale = component.clientWidth / component.clientHeight;
      console.log('scale: ', setup.scale);
    }

    const viewport = page.getViewport({
      scale: setup.scale,
      rotation: setup.rotation,
    });

    const { height: height, width: width } = viewport;
    renderCanvas.height = height;
    renderCanvas.width = width;
    selectedCanvas.height = height;
    selectedCanvas.width = width;
    processingCanvas.height = height;
    processingCanvas.width = width;

    const canvasContext = renderCanvas.getContext('2d');
    const selectedCanvasContext = selectedCanvas.getContext('2d');
    const processingCanvasContext = processingCanvas.getContext('2d');

    if (canvasContext && selectedCanvasContext && processingCanvasContext) {
      const fontSize = Math.min(width, height) * 0.05;
      const text = `Página ${pageNumber} selecionada`;
      const text2 = `Página ${pageNumber} em processamento...`;
      selectedCanvasContext.font = `bold ${fontSize}px Arial`;
      processingCanvasContext.font = `bold ${fontSize}px Arial`;
      const textWidth = selectedCanvasContext.measureText(text).width;
      const textWidth2 = processingCanvasContext.measureText(text2).width;

      const x = (width - textWidth) / 2;
      const y = (height + fontSize) / 2 - fontSize / 2;
      const x2 = (width - textWidth2) / 2;
      const y2 = (height + fontSize) / 2 - fontSize / 2;

      selectedCanvasContext.fillStyle = 'rgba(232, 227, 229, 0.7)';
      selectedCanvasContext.fillRect(0, 0, width, height);
      selectedCanvasContext.fillStyle = 'rgba(186, 79, 125, 1)';
      selectedCanvasContext.fillText(text, x, y);

      processingCanvasContext.fillStyle = 'rgba(186, 79, 125, 0.7)';
      processingCanvasContext.fillRect(0, 0, width, height);
      processingCanvasContext.fillStyle = 'rgba(232, 227, 229, 1)';
      processingCanvasContext.fillText(text2, x2, y2);

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
    setup.scale = Math.max(0.4, Math.min(10, setup.scale! * scaleFactor));
  };

  const updateRotation = (delta: number) => {
    setup.rotation = (((setup.rotation + delta) % 360) + 360) % 360;
  };

  const onFirstPage = () => (setup.pageNumber = 1);
  const onLastPage = () => (setup.pageNumber = setup.numPages!);
  const onPrevPage = () => updatePageNumber(-1);
  const onNextPage = () => updatePageNumber(1);
  const onZoomIn = () => updateScale(0.4);
  const onZoomOut = () => updateScale(-0.4);
  const onRotateLeft = () => updateRotation(-90);
  const onRotateRight = () => updateRotation(90);
  const onWheel = (e: WheelEvent) => {
    if (e.ctrlKey) {
      e.deltaY < 0 ? updateScale(0.1) : updateScale(-0.1);
    } else if (e.shiftKey) {
      e.deltaY < 0 ? updateRotation(90) : updateRotation(-90);
    }
  };
  const onSelectPDF = async () => {
    const file = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
      title: 'Por favor, selecione um PDF',
      defaultPath: await homeDir(),
    });
    if (!file) return;
    setup.path = file.path;
  };

  const onSelectPage = (pageNumber: number) => {
    if (processingPages.includes(pageNumber)) return;
    const index = selectedPages.indexOf(pageNumber);
    if (index !== -1) {
      selectedPages.splice(index, 1);
      const prevSelectedPage = selectedPages
        .slice()
        .reverse()
        .find((page) => page < pageNumber && !processingPages.includes(page));
      if (prevSelectedPage) {
        setup.pageNumber = prevSelectedPage;
      } else {
        const nextSelectedPage = selectedPages.find(
          (page) => page > pageNumber && !processingPages.includes(page),
        );
        if (nextSelectedPage) {
          setup.pageNumber = nextSelectedPage;
        }
      }
    } else {
      selectedPages.push(pageNumber);
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
      if (prevUnselectedPage) {
        setup.pageNumber = prevUnselectedPage;
      } else {
        const nextUnselectedPage = Array.from(
          { length: setup.numPages! },
          (_, i) => i + 1,
        ).find(
          (page) =>
            page > pageNumber &&
            !selectedPages.includes(page) &&
            !processingPages.includes(page),
        );
        if (nextUnselectedPage) {
          setup.pageNumber = nextUnselectedPage;
        }
      }
    }
    selectedPages.sort((a, b) => a - b);
    console.log('Páginas selecionadas: ' + selectedPages);
  };

  const onSelectPageRepresentation = (pageNumber: number) => {
    if (setup.pageNumber === pageNumber) {
      onSelectPage(pageNumber);
    } else {
      setup.pageNumber = pageNumber;
    }
  };

  $effect(() => {
    console.log('pdfjs-dist version: ' + pdfjs.version);
    if (!setup.path) return;
    loadDocument()
      .then((document) => {
        untrack(() => {
          selectedPages.splice(0, selectedPages.length);
        });
        setup.document = document;
        setup.numPages = document.numPages;
        setup.pageNumber = 1;
        document.getMetadata().then((metadata) => {
          console.log('Document loaded!\nMetadata:\n', metadata);
        });
      })
      .catch((error) => console.error(error));
    return () => setup.document?.destroy();
  });

  $effect(() => {
    if (!setup.document) return;
    setup.scale;
    setup.rotation;
    if (setup.pageNumber > 0 && setup.pageNumber <= setup.numPages!) {
      tick().then(() => {
        loadPageQueue(setup.pageNumber);
        console.log('Page loaded!\nPage number: ' + setup.pageNumber);
      });
    }
  });

  const onProcessPages = () => {
    processingPages.push(...selectedPages);
    selectedPages.splice(0, selectedPages.length);
    console.log('Páginas processadas: ' + processingPages);
  };

  const dialogText = () => {
    let output;
    if (selectedPages.length === 1) {
      output = `Página ${selectedPages[0]} selecionada`;
    } else if (selectedPages.length === 2) {
      output = `Páginas ${selectedPages[0]} e ${selectedPages[1]} selecionadas`;
    } else {
      output = 'Páginas selecionadas: ';
      for (let i = 0; i < selectedPages.length; i++) {
        if (i === selectedPages.length - 1) {
          output += `e ${selectedPages[i]}`;
        } else {
          output += `${selectedPages[i]}, `;
        }
      }
    }
    return output + '.';
  };
</script>

<svelte:window
  onmousedown={(e) => {
      if (e.detail > 1) {
          e.preventDefault()
        }
      if (!component.contains(e.target as Node) && !canvasContainer.contains(e.target as Node)) {
        setup.isActive = false
      } else {
        setup.isActive = true
      }
    }}
  ondblclick={(e) => {
      if (!setup.numPages || !canvasContainer.contains(e.target as Node)) return;
      setup.numPages && onSelectPage(setup.pageNumber)
    }}
  onwheel={(e) => {
      if (!setup.numPages) return
      if (canvasContainer.contains(e.target as Node)) {
        setup.isActive = true
        onWheel(e);
      } else if (component.contains(e.target as Node)) {
        setup.isActive = true
        if (e.deltaY < 0) {
          onPrevPage();
        } else {
          onNextPage();
        }
      } else {
        setup.isActive = false
      }
    }}
  onkeydown={(e) => {
    if (!setup.isActive) return;
    e.key === 'Escape' && onSelectPDF();
    if (!setup.numPages) return;
    e.key === 'ArrowLeft' && onPrevPage();
    e.key === 'ArrowRight' && onNextPage();
    e.key === ' ' && (e.preventDefault(), onSelectPage(setup.pageNumber));
    e.key === 'Home' && onFirstPage();
    e.key === 'End' && onLastPage();
    e.shiftKey && e.key === 'ArrowLeft' && onFirstPage();
    e.shiftKey && e.key === 'ArrowRight' && onLastPage();
    e.shiftKey && e.key === 'ArrowUp' && onZoomIn();
    e.shiftKey && e.key === 'ArrowDown' && onZoomOut();
    e.ctrlKey && e.key === '=' && onZoomIn();
    e.ctrlKey && e.key === '-' && onZoomOut();
    e.ctrlKey && e.key === 'ArrowLeft' && onRotateLeft();
    e.ctrlKey && e.key === 'ArrowRight' && onRotateRight();
    if (!selectedPages.length) return;
    if (e.key === 'Backspace') setup.pageNumber = selectedPages.pop()!;
    if (e.key === 'Enter' && !setup.confirmProcessDialogOpen)
      setup.confirmProcessDialogOpen = true;
  }}
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
      <svg class="absolute left-0 top-0" bind:this={textLayerSvg}></svg>
      <canvas
        bind:this={selectedCanvas}
        class:hidden={!selectedPages.includes(setup.pageNumber) ||
          processingPages.includes(setup.pageNumber)}
        class="pointer-events-none absolute left-0 top-0"
      ></canvas>
      <canvas
        bind:this={processingCanvas}
        class:hidden={!processingPages.includes(setup.pageNumber) ||
          selectedPages.includes(setup.pageNumber)}
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
          on:click={() => onSelectPageRepresentation(page)}>{page}</Button
        >
      {/each}
    </div>
  {/if}

  <div class="absolute right-4 top-4 flex flex-col space-y-2">
    <Button
      tabindex={-1}
      size="icon"
      on:click={onRotateLeft}
      disabled={!setup.numPages}><RotateCcw /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      on:click={onRotateRight}
      disabled={!setup.numPages}><RotateCw /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      on:click={onZoomIn}
      disabled={!setup.numPages || setup.scale === 10}><ZoomIn /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      on:click={onZoomOut}
      disabled={!setup.numPages || setup.scale === 0.5}><ZoomOut /></Button
    >
  </div>

  <Button
    tabindex={-1}
    class="absolute bottom-4 left-4 "
    size="icon"
    on:click={onSelectPDF}><FolderOpen /></Button
  >

  <div
    class="absolute bottom-4 left-1/2 flex -translate-x-1/2 scale-90 transform items-center justify-center space-x-2"
  >
    <Button
      tabindex={-1}
      size="icon"
      on:click={onFirstPage}
      disabled={!setup.numPages || setup.pageNumber === 1}
      ><ChevronFirst /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      on:click={onPrevPage}
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
      on:click={onNextPage}
      disabled={!setup.numPages || setup.pageNumber === setup.numPages}
      ><ArrowRight /></Button
    >
    <Button
      tabindex={-1}
      size="icon"
      on:click={onLastPage}
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
            {dialogText()}
          </Dialog.Description>
        </Dialog.Header>
        <Dialog.Footer>
          <Button
            on:click={() => {
              onProcessPages();
              setup.confirmProcessDialogOpen = false;
            }}>Processar</Button
          >
        </Dialog.Footer>
      </Dialog.Content>
    </Dialog.Root>

    <Button
      tabindex={-1}
      size="icon"
      on:click={(e) => (
        setup.isActive && e.stopImmediatePropagation(),
        onSelectPage(setup.pageNumber)
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
