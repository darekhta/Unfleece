declare module 'tesseract-wasm' {
  export interface IntRect {
    left: number;
    top: number;
    right: number;
    bottom: number;
  }

  export interface TextItem {
    rect: IntRect;
    flags: number;
    confidence: number;
    text: string;
  }

  export type TextUnit = 'line' | 'word';
  export type ProgressListener = (progress: number) => void;

  export interface OCRClientInit {
    createWorker?: (url: string) => Worker;
    wasmBinary?: Uint8Array | ArrayBuffer;
    workerURL?: string;
  }

  export class OCRClient {
    constructor(init?: OCRClientInit);
    destroy(): Promise<void>;
    loadModel(model: string | ArrayBuffer): Promise<void>;
    loadImage(image: ImageBitmap | ImageData): Promise<void>;
    clearImage(): Promise<void>;
    getTextBoxes(unit: TextUnit, onProgress?: ProgressListener): Promise<TextItem[]>;
    getText(onProgress?: ProgressListener): Promise<string>;
  }

  export function supportsFastBuild(): boolean;
}
