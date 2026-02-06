import { useKeyPress } from "ahooks";
import clsx from "clsx";
import { useContext } from "react";
import UnoIcon from "@/components/UnoIcon";
import { PRESET_SHORTCUT } from "@/constants";
import { useTauriFocus } from "@/hooks/useTauriFocus";
import { hideWindow, setWindowPinned } from "@/plugins/window";
import { MainContext } from "../..";

const WindowPin = () => {
  const { rootState } = useContext(MainContext);

  useKeyPress(PRESET_SHORTCUT.FIXED_WINDOW, () => {
    togglePin();
  });

  useTauriFocus({
    onBlur() {
      if (rootState.pinned) return;

      hideWindow();
    },
  });

  const togglePin = () => {
    const nextPinned = !rootState.pinned;

    rootState.pinned = nextPinned;
    setWindowPinned(nextPinned);
  };

  return (
    <UnoIcon
      active={rootState.pinned}
      className={clsx({ "-rotate-45": !rootState.pinned })}
      hoverable
      name="i-lets-icons:pin"
      onMouseDown={togglePin}
    />
  );
};

export default WindowPin;
