import type { ProcessingPage, ProcessedDocument } from "$lib/types";

export type DocumentState = {
  selectedPages: number[];
  processingPages: ProcessingPage[];
  processedDocuments: ProcessedDocument[];
};

export function createDocumentState(): DocumentState {
  let selectedPages = $state<number[]>([]);
  let processingPages = $state<ProcessingPage[]>([]);
  let processedDocuments = $state<ProcessedDocument[]>([]);

  return {
    get selectedPages() {
      return selectedPages;
    },
    set selectedPages(value) {
      selectedPages = value;
    },
    get processingPages() {
      return processingPages;
    },
    set processingPages(value) {
      processingPages = value;
    },
    get processedDocuments() {
      return processedDocuments;
    },
    set processedDocuments(value) {
      processedDocuments = value;
    },
  };
}
