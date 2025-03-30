import { Theme } from "./theme";
import { useCallback, useEffect, useState } from "react";
import { editor, Range } from "monaco-editor";
import IStandaloneCodeEditor = editor.IStandaloneCodeEditor;
import MonacoEditor from "@monaco-editor/react";

// TODO: Add another Tool with fixed text

export enum SecondaryTool {
  "Tokens" = "Tokens",
}

export type SecondaryPanelResult = {
  content: string
}

export type SecondaryPanelProps = {
  tool: SecondaryTool;
  result: SecondaryPanelResult;
  theme: Theme;
};

export default function SecondaryPanel({
  tool,
  result,
  theme,
}: SecondaryPanelProps) {
  return (
    <div className="flex flex-col h-full">
      <div className="flex-grow">
        <Content
          tool={tool}
          result={result}
          theme={theme}
        />
      </div>
    </div>
  );
}

function Content({
  tool,
  result,
  theme,
}: SecondaryPanelProps) {
  // Note that 70%+ of this function is only useful if I ever want
  // to implement the GoTo from range to text. Otherwise I could purge it.
  //
  // Tool is only Token for the moment...
  // This is just here to prevent unused message
  console.log(tool);

  const [editor, setEditor] = useState<IStandaloneCodeEditor | null>(null);
  const [ranges, setRanges] = useState<
    Array<{ byteRange: { start: number; end: number }; textRange: Range }>
  >([]);


  useEffect(() => {
    const model = editor?.getModel();
    if (editor == null || model == null) {
      return;
    }

    const handler = editor.onMouseDown((event) => {
      if (event.target.range == null) {
        return;
      }

      const range = model
        .getDecorationsInRange(
          event.target.range,
          undefined,
          true,
          false,
          false,
        )
        .map((decoration) => {
          const decorationRange = decoration.range;
          return ranges.find((range) =>
            Range.equalsRange(range.textRange, decorationRange),
          );
        })
        .find((range) => range != null);

      if (range == null) {
        return;
      }
    });

    return () => handler.dispose();
  }, [editor, ranges]);

  const handleDidMount = useCallback((editor: IStandaloneCodeEditor) => {
    setEditor(editor);

    const model = editor.getModel();
    const collection = editor.createDecorationsCollection([]);

    function updateRanges() {
      if (model == null) {
        setRanges([]);
        collection.set([]);
        return;
      }

      const matches = model.findMatches(
        String.raw`(\d+)\.\.(\d+)`,
        false,
        true,
        false,
        ",",
        true,
      );

      const ranges = matches
        .map((match) => {
          const startByteOffset = parseInt(match.matches![1] ?? "", 10);
          const endByteOffset = parseInt(match.matches![2] ?? "", 10);

          if (Number.isNaN(startByteOffset) || Number.isNaN(endByteOffset)) {
            return null;
          }

          return {
            byteRange: { start: startByteOffset, end: endByteOffset },
            textRange: match.range,
          };
        })
        .filter((range) => range != null);

      setRanges(ranges);

      const decorations = ranges.map((range) => {
        return {
          range: range.textRange,
          options: {
            inlineClassName:
              "underline decoration-slate-600 decoration-1 cursor-pointer",
          },
        };
      });

      collection.set(decorations);
    }

    updateRanges();
    const handler = editor.onDidChangeModelContent(updateRanges);

    return () => handler.dispose();
  }, []);

  if (result == null) {
    return "";
  } else {
    return (
      <MonacoEditor
        options={{
          readOnly: true,
          minimap: { enabled: false },
          fontSize: 14,
          roundedSelection: false,
          scrollBeyondLastLine: false,
          contextmenu: false,
          // Custom
          unicodeHighlight: {
            ambiguousCharacters: false,
            invisibleCharacters: false,
            nonBasicASCII: false,
          },
        }}
        onMount={handleDidMount}
        value={result.content}
        theme={theme === "light" ? "vs" : "vs-dark"}
      />
    );
  }
}
