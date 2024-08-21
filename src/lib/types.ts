import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist";

export interface FileNameGeneration {
  file_name: string;
  file_name_history: string[];
  json_file_path: string;
  reasoning: {
    document_summary: {
      analysis: string;
      formatting_process: string;
      summary: string;
    };
    document_type: {
      analysis: string;
      type_name: string;
    };
    important_date: {
      analysis: string;
      date: string;
    };
    language: string;
    main_entities: {
      analysis: string;
      entities: string;
    };
    type_abbreviation: {
      analysis: string;
      type_abbr: string;
    };
  };
}

export interface ProcessingPage {
  id: string;
  pages: number[];
  status: "pending" | "processing" | "completed" | "error";
  startTime: number;
  endTime?: number;
  elapsed?: number;
  error?: string;
  json_file_path?: string;
}

export interface ProcessedDocument extends ProcessingPage {
  file_name: string;
  debug: FileNameGeneration;
}

export interface SetupState {
  path: string | undefined;
  dataPath: string | undefined;
  document: PDFDocumentProxy | undefined;
  page: PDFPageProxy | undefined;
  scale: number;
  rotation: number;
  numPages: number;
  pageNumber: number;
  pageRendering: boolean;
  pageNumPending: number | undefined;
  metadata: any | undefined;
  isActive: boolean;
  confirmProcessDialogOpen: boolean;
}
