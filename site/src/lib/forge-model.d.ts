// Generated from forge_spec — regenerate with: forge schema > static/schema/forge-doc.schema.json && npx json-schema-to-typescript

export type Value =
  | {
      String: string;
    }
  | {
      Interpolated: Segment[];
    }
  | {
      Number: {
        value: number;
        unit?: Unit | null;
        [k: string]: unknown;
      };
    }
  | {
      Bool: boolean;
    }
  | {
      Enum: string;
    }
  | {
      Color: Color;
    }
  | {
      Token: Path;
    }
  | {
      List: Value[];
    }
  | {
      Map: [unknown, unknown][];
    }
  | {
      Binding: {
        path: Path;
        mode: BindingMode;
        [k: string]: unknown;
      };
    };
export type Segment =
  | {
      Text: string;
    }
  | {
      Binding: Path;
    };
export type PathSegment =
  | {
      Field: string;
    }
  | {
      Index: number;
    };
/**
 * A path into the view state, e.g. `form.items[0].name`.
 */
export type Path = PathSegment[];
export type Unit = "Px" | "Rem" | "Percent" | "Fr";
export type BindingMode = "OneWay" | "TwoWay";

export interface Document {
  /**
   * JSON Schema URL for editors (`$schema`).
   */
  schema?: string | null;
  version: string;
  /**
   * Base component system; inherited from `forge.toml` when absent.
   */
  base?: string | null;
  imports: Import[];
  meta: [unknown, unknown][];
  /**
   * Top-level `Dialog#name:` declarations, opened by `open_dialog(name)`.
   */
  dialogs: Node[];
  root: Node;
  [k: string]: unknown;
}
export interface Import {
  alias: string;
  library: string;
  version?: string | null;
  [k: string]: unknown;
}
export interface Node {
  element: ElementRef;
  id?: string | null;
  /**
   * Positional value; the catalog maps it to the element's primary prop.
   */
  content?: Value | null;
  props: [unknown, unknown][];
  style: [unknown, unknown][];
  events: EventBinding[];
  children: Node[];
  [k: string]: unknown;
}
export interface ElementRef {
  /**
   * Import alias; `None` is the base library.
   */
  namespace?: string | null;
  name: string;
  [k: string]: unknown;
}
export interface Color {
  r: number;
  g: number;
  b: number;
  a: number;
  [k: string]: unknown;
}
export interface EventBinding {
  event: string;
  handler: HandlerRef;
  [k: string]: unknown;
}
/**
 * A hook id resolved by the hook registry at runtime, with arguments.
 */
export interface HandlerRef {
  name: string;
  args: Value[];
  [k: string]: unknown;
}
