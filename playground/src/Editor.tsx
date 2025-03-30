import { useRef, useState, useCallback, useEffect } from 'react'
import { Panel, PanelGroup } from "react-resizable-panels";
import { Diagnostic, Token, scan_text, tokenize, syllabify } from './../pkg/grs_wasm'
import { Source } from './App';
import { Theme } from './theme';
import PrimarySideBar from "./PrimarySideBar";
import SecondarySideBar from "./SecondarySideBar";
import SecondaryPanel, {
  SecondaryPanelResult,
  SecondaryTool,
} from "./SecondaryPanel";
import { HorizontalResizeHandle } from "./ResizeHandle";

type Tab = "Source" | "Settings";

interface EditorProps {
  source: Source;
  theme: Theme;

  onSourceChanged(source: string): void;
  onSettingsChanged(settings: string): void;
}

export default function Editor({
  source,
  theme,
  onSourceChanged,
  onSettingsChanged
}: EditorProps) {
  const editorRef = useRef<IStandaloneCodeEditor | null>(null);
  const [tab, setTab] = useState<Tab>("Source");
  const [secondaryTool, setSecondaryTool] = useState<SecondaryTool | null>(
    () => {
      const secondaryValue = new URLSearchParams(location.search).get("secondary");
      // .prototype.hasOwnProperty.call ~~ hasOwn
      return secondaryValue && Object.prototype.hasOwnProperty.call(SecondaryTool, secondaryValue)
        ? (secondaryValue as SecondaryTool)
        : null;
    },
  );
  const handleSecondaryToolSelected = (tool: SecondaryTool | null) => {
    if (tool === secondaryTool) {
      tool = null;
    }

    const url = new URL(location.href);

    if (tool == null) {
      url.searchParams.delete("secondary");
    } else {
      url.searchParams.set("secondary", tool);
    }

    history.replaceState(null, "", url);

    setSecondaryTool(tool);
  };

  const handleSourceEditorMount = useCallback(
    (editor: IStandaloneCodeEditor) => {
      editorRef.current = editor;

      editor.addAction({
        contextMenuGroupId: "navigation",
        contextMenuOrder: 0,
        id: "reveal-node",
        label: "Reveal node",
        precondition: "editorTextFocus",

        run(editor: editor.ICodeEditor): void | Promise<void> {
          const position = editor.getPosition();
          if (position == null) {
            return;
          }
        },
      });
    },
    [],
  );

  // Settings
  const { settings } = source;
  let config;
  try {
    config = JSON.parse(settings);
  } catch {
    config = {};
  }

  const diagnostics = scan_text(source.text, config);
  const tokens: Token[] = tokenize(source.text);
  // const tokens_str = JSON.stringify(tokens, null, 2);
  const tokens_str = tokens.map(token =>
    `${JSON.stringify(token.text)}, ${JSON.stringify(token.whitespace)}, ${token.range.start}..${token.range.end}`
  ).join("\n");
  const result: SecondaryPanelResult = {
    content: tokens_str,
  };

  return (
    <PanelGroup direction="horizontal" autoSaveId="main">
      <PrimarySideBar onSelectTool={(tool) => setTab(tool)} selected={tab} />

      <Panel id="main" order={0} minSize={10}>
        <PanelGroup id="main" direction="vertical">
          <SourceEditor
            visible={tab === "Source"}
            source={source.text}
            diagnostics={diagnostics}
            theme={theme}
            onChange={onSourceChanged}
            onMount={handleSourceEditorMount}
          />
          <SettingsEditor
            visible={tab === "Settings"}
            source={source.settings}
            theme={theme}
            onChange={onSettingsChanged}
          />
        </PanelGroup>
      </Panel>

      {secondaryTool != null && (
        <>
          <HorizontalResizeHandle />
          <Panel
            id="secondary-panel"
            order={1}
            className={"my-2"}
            minSize={10}
          >
            <SecondaryPanel
              theme={theme}
              tool={secondaryTool}
              result={result}
            />
          </Panel>
        </>
      )}

      <SecondarySideBar
        selected={secondaryTool}
        onSelected={handleSecondaryToolSelected}
      />
    </PanelGroup>
  );
}


import MonacoEditor, { Monaco, OnMount } from "@monaco-editor/react";
import {
  editor,
  MarkerSeverity,
} from "monaco-editor";
import IStandaloneCodeEditor = editor.IStandaloneCodeEditor;

type MonacoEditorState = {
  monaco: Monaco;
};

function SourceEditor({
  visible,
  source,
  theme,
  diagnostics,
  onChange,
  onMount,
}: {
  visible: boolean;
  source: string;
  diagnostics: Diagnostic[];
  theme: Theme;
  onChange(pythonSource: string): void;
  onMount(editor: IStandaloneCodeEditor): void;
}) {
  const monacoRef = useRef<MonacoEditorState | null>(null);

  // Update the diagnostics in the editor.
  useEffect(() => {
    const editorState = monacoRef.current;
    if (editorState == null) {
      return;
    }
    updateMarkers(editorState.monaco, diagnostics);
  }, [diagnostics]);

  const handleChange = useCallback(
    (value: string | undefined) => {
      onChange(value ?? "");
    },
    [onChange],
  );

  const handleMount: OnMount = useCallback(
    (editor, instance) => {
      updateMarkers(instance, diagnostics);
      monacoRef.current = {
        monaco: instance,
      };
      onMount(editor);
    },

    [diagnostics, onMount],
  );

  return (
    <MonacoEditor
      onMount={handleMount}
      options={{
        fixedOverflowWidgets: true,
        readOnly: false,
        minimap: { enabled: false },
        fontSize: 16,
        roundedSelection: false,
        scrollBeyondLastLine: false,
        contextmenu: true,
        /* Custom */
        unicodeHighlight: {
          ambiguousCharacters: false,
          invisibleCharacters: false,
          nonBasicASCII: false,
        },
        wordWrap: "on",
      }}
      wrapperProps={visible ? {} : { style: { display: "none" } }}
      theme={theme === "light" ? "vs" : "vs-dark"}
      value={source}
      onChange={handleChange}
    />
  );
}

interface ExpandedDiagnostic {
  kind: string;
  range: string;
  fix: string;
  syllables?: string;
};

const ACCENT_RELATED_DIAGNOSTIC_KINDS = [
  "missing_double_accents",
  "multisyllable_not_accented",
  // monosyllable is redundant, since we already know the syllabification
];

// Does two things on a diagnostic:
// * Replace the range attribute with a more compact string
// * For accent related diagnostics, add the syllabification on hover
function toExpandedDiagnostic(diagnostic: Diagnostic, word: string): ExpandedDiagnostic {
  const expandedDiagnostic: ExpandedDiagnostic = {
    kind: diagnostic.kind,
    range: `${diagnostic.range.start}..${diagnostic.range.end}`,
    fix: diagnostic.fix,
  };

  if (ACCENT_RELATED_DIAGNOSTIC_KINDS.includes(diagnostic.kind)) {
    expandedDiagnostic.syllables = syllabify(word, "-");
  }

  return expandedDiagnostic
}

function updateMarkers(monaco: Monaco, diagnostics: Array<Diagnostic>) {
  const editor = monaco.editor;
  const model = editor?.getModels()[0];

  if (!model) {
    return;
  }

  editor.setModelMarkers(
    model,
    "owner",
    diagnostics.map((diagnostic) => {
      const start = model.getPositionAt(diagnostic.range.start);
      const end = model.getPositionAt(diagnostic.range.end);

      // Get back the word from the diagnostic
      const range = new monaco.Range(
        start.lineNumber,
        start.column,
        end.lineNumber,
        end.column
      );
      const word = model.getValueInRange(range);

      const expandedDiagnostic = toExpandedDiagnostic(diagnostic, word);

      return (
        {
          startLineNumber: start.lineNumber,
          startColumn: start.column,
          endLineNumber: end.lineNumber,
          endColumn: end.column,
          message: JSON.stringify(expandedDiagnostic, null, 2),
          severity: MarkerSeverity.Error,
        })
    }),
  );
}

function SettingsEditor({
  visible,
  source,
  theme,
  onChange,
}: {
  visible: boolean;
  source: string;
  theme: Theme;
  onChange: (source: string) => void;
}) {
  const handleChange = useCallback(
    (value: string | undefined) => {
      onChange(value ?? "");
    },
    [onChange],
  );

  const handleMount = useCallback((editor: IStandaloneCodeEditor) => {
    const model = editor.getModel();

    if (model == null) {
      return;
    }
  }, []);

  return (
    <MonacoEditor
      options={{
        readOnly: false,
        minimap: { enabled: false },
        fontSize: 20,
        roundedSelection: false,
        scrollBeyondLastLine: false,
        contextmenu: true,
      }}
      onMount={handleMount}
      wrapperProps={visible ? {} : { style: { display: "none" } }}
      language="json"
      value={source}
      theme={theme === "light" ? "vs" : "vs-dark"}
      onChange={handleChange}
    />
  );
}
