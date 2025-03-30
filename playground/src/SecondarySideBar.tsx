import SideBar, { SideBarEntry } from "./SideBar";
import { TokensIcon } from "./Icons";
import { SecondaryTool } from "./SecondaryPanel";

interface RightSideBarProps {
  selected: SecondaryTool | null;
  onSelected(tool: SecondaryTool): void;
}

export default function SecondarySideBar({
  selected,
  onSelected,
}: RightSideBarProps) {
  return (
    <SideBar position="right">
      <SideBarEntry
        title="Tokens"
        position={"right"}
        selected={selected === SecondaryTool.Tokens}
        onClick={() => onSelected(SecondaryTool.Tokens)}
      >
        <TokensIcon />
      </SideBarEntry>
    </SideBar>
  );
}
