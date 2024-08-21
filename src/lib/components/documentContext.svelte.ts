import type { ProcessingPage, ProcessedDocument } from "$lib/types";

export type DocumentState = {
  currentPageNumber: number;
  selectedPages: number[];
  processingPages: ProcessingPage[];
  processedDocuments: ProcessedDocument[];
  finishedDocuments: ProcessedDocument[];
};

export function createDocumentState(): DocumentState {
  let currentPageNumber = $state<number>(1);
  let selectedPages = $state<number[]>([]);
  let processingPages = $state<ProcessingPage[]>([]);
  let processedDocuments = $state<ProcessedDocument[]>([]);
  let finishedDocuments = $state<ProcessedDocument[]>([]);
  return {
    get currentPageNumber() {
      return currentPageNumber;
    },
    set currentPageNumber(value) {
      currentPageNumber = value;
    },
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
    get finishedDocuments() {
      return finishedDocuments;
    },
    set finishedDocuments(value) {
      finishedDocuments = value;
    },
  };
}