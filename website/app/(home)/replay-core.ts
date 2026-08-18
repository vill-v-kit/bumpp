/**
 * 重放式 seek 的终端核心：wterm 的 TerminalCore 没有
 * reset/snapshot/seek，本类在 WasmBridge 上扩出 replay——「init 重置 wasm
 * 核心 + 同步写入全部 ≤t 字节」，之后渲染层下一次绘制读取的就是重放后
 * 的屏幕状态。WasmBridge.init 重建网格但脏行标记语义不由我们控制，
 * 故 seek 后自行把全部行标脏一次，保证回退时被清空的行也会重绘。
 */
import { WasmBridge } from '@wterm/core';

export class ReplayCore extends WasmBridge {
  private cols = 80;
  private rows = 24;
  private dirtyAll = false;

  private constructor(instance: WebAssembly.Instance) {
    super(instance);
  }

  static override async load(wasmUrl: string): Promise<ReplayCore> {
    // 与 WasmBridge.load 同路的 fetch + 实例化，产出本子类
    //（基类静态 load 的返回类型钉死在 WasmBridge，无法复用）
    const response = await fetch(wasmUrl);
    if (!response.ok) {
      throw new Error(
        `[wterm] Failed to load WASM from ${wasmUrl}: ${response.status} ${response.statusText}`,
      );
    }
    const { instance } = await WebAssembly.instantiate(
      await response.arrayBuffer(),
    );
    return new ReplayCore(instance);
  }

  override init(cols: number, rows: number): void {
    this.cols = cols;
    this.rows = rows;
    super.init(cols, rows);
  }

  override resize(cols: number, rows: number): void {
    this.cols = cols;
    this.rows = rows;
    super.resize(cols, rows);
  }

  /** 重放式 seek：重置核心并同步写入前缀字节，随后全行标脏一次 */
  replay(text: string): void {
    super.init(this.cols, this.rows);
    super.writeString(text);
    this.dirtyAll = true;
  }

  override isDirtyRow(row: number): boolean {
    return this.dirtyAll || super.isDirtyRow(row);
  }

  override clearDirty(): void {
    this.dirtyAll = false;
    super.clearDirty();
  }
}
