"""Totally definitely EDMCOverlay."""

import logging
from pathlib import Path
import tkinter as tk
from tkinter import ttk

from config import appname, config
import myNotebook as nb
import plug
from ttkHyperlinkLabel import HyperlinkLabel


plugin_name = Path(__file__).parent.name
logger = logging.getLogger(f"{appname}.{plugin_name}")

logger.info(r"""edmcoverlay Crab Edition: starting up
  (<      >)  (<      >)  (<      >)
   `O,99,O`    `O,99,O`    `O,99,O`
  //-\__/-\\  //-\__/-\\  //-\__/-\\  ldb""")

logger.debug("edmcoverlay CE: loading plugin, importing lib")
import edmcoverlay
logger.debug("edmcoverlay CE: got lib: %s", repr(edmcoverlay))
import edmcoverlay._edmcoverlay
logger.debug("edmcoverlay CE: got internal lib: %s", repr(edmcoverlay._edmcoverlay))

xpos_var: tk.IntVar
ypos_var: tk.IntVar
width_var: tk.IntVar
height_var: tk.IntVar


def plugin_start3(plugin_dir):
    logger.info("edmcoverlay CE: plugin start!")
    return "edmcoverlay CE"


def journal_entry(cmdr, is_beta, system, station, entry, state):
    if entry["event"] in ["LoadGame", "StartUp"]:
        logger.info("edmcoverlay CE: load event received, starting overlay")
        edmcoverlay._edmcoverlay.ensure_overlay()
    elif entry["event"] in ["Shutdown", "ShutDown"]:
        logger.info("edmcoverlay CE: shutdown event received, stopping overlay")
        edmcoverlay._edmcoverlay.stop_overlay()


def plugin_stop():
    global overlay_process
    logger.info("edmcoverlay CE: exiting plugin")
    edmcoverlay._edmcoverlay.stop_overlay()


def plugin_prefs(parent: nb.Notebook, cmdr: str, is_beta: bool) -> nb.Frame:
    global xpos_var, ypos_var, width_var, height_var
    xpos_var = tk.IntVar(value=int(config.get("edmcoverlay2_xpos") or 0))
    ypos_var = tk.IntVar(value=int(config.get("edmcoverlay2_ypos") or 0))
    width_var = tk.IntVar(value=int(config.get("edmcoverlay2_width") or 1920))
    height_var = tk.IntVar(value=int(config.get("edmcoverlay2_height") or 1080))
    frame = nb.Frame(parent)
    frame.columnconfigure(0, weight=1)
    PAD_X = 10
    PAD_Y = 2

    f0 = nb.Frame(frame)
    HyperlinkLabel(f0, text="edmcoverlay Crab Edition", url="https://github.com/sersorrel/edmcoverlay.rs", background=nb.Label().cget('background'), underline=True).grid(row=0, column=0, sticky=tk.W, padx=(PAD_X, 0))
    nb.Label(f0, text="by Ash Holland").grid(row=0, column=1, sticky=tk.W, padx=(0, PAD_X))
    f0.grid(sticky=tk.EW)

    ttk.Separator(frame, orient=tk.HORIZONTAL).grid(padx=PAD_X, pady=2 * PAD_Y, sticky=tk.EW)

    f1 = nb.Frame(frame)
    nb.Label(f1, text="Overlay configuration:").grid(row=0, column=0, columnspan=3, padx=PAD_X, pady=PAD_Y, sticky=tk.W)
    nb.Label(f1, text="X position").grid(row=1, column=0, padx=PAD_X, pady=(PAD_Y, 0), sticky=tk.E)
    nb.Entry(f1, textvariable=xpos_var).grid(row=1, column=1, columnspan=3, padx=(0, PAD_X), pady=PAD_Y, sticky=tk.W)
    nb.Label(f1, text="Y position").grid(row=2, column=0, padx=PAD_X, pady=(PAD_Y, 0), sticky=tk.E)
    nb.Entry(f1, textvariable=ypos_var).grid(row=2, column=1, columnspan=3, padx=(0, PAD_X), pady=PAD_Y, sticky=tk.W)
    nb.Label(f1, text="Width").grid(row=3, column=0, padx=PAD_X, pady=(PAD_Y, 0), sticky=tk.E)
    nb.Entry(f1, textvariable=width_var).grid(row=3, column=1, columnspan=3, padx=(0, PAD_X), pady=PAD_Y, sticky=tk.W)
    nb.Label(f1, text="Height").grid(row=4, column=0, padx=PAD_X, pady=(PAD_Y, 0), sticky=tk.E)
    nb.Entry(f1, textvariable=height_var).grid(row=4, column=1, columnspan=3, padx=(0, PAD_X), pady=PAD_Y, sticky=tk.W)
    f1.grid(sticky=tk.EW)

    ttk.Separator(frame, orient=tk.HORIZONTAL).grid(padx=PAD_X, pady=2 * PAD_Y, sticky=tk.EW)

    f2 = nb.Frame(frame)
    nb.Label(f2, text="Manual overlay controls:").grid(row=0, column=0, padx=PAD_X, pady=PAD_Y)
    nb.Button(f2, text="Start overlay", command=lambda: edmcoverlay._edmcoverlay.start_overlay()).grid(row=0, column=1, padx=PAD_X, pady=PAD_Y)
    nb.Button(f2, text="Stop overlay", command=lambda: edmcoverlay._edmcoverlay.stop_overlay()).grid(row=0, column=2, padx=PAD_X, pady=PAD_Y)
    f2.grid(sticky=tk.EW)

    return frame


def prefs_changed(cmdr: str, is_beta: bool) -> None:
    xpos = xpos_var.get()
    ypos = ypos_var.get()
    width = width_var.get()
    height = height_var.get()
    change = False
    for name, val in [("xpos", xpos), ("ypos", ypos), ("width", width), ("height", height)]:
        try:
            assert int(val) >= 0
        except (ValueError, AssertionError):
            logger.warning("Bad config value for %s: %r", name, val)
        else:
            try:
                old_val = int(config.get(f"edmcoverlay2_{name}"))
            except (TypeError, ValueError):
                pass
            else:
                if val != old_val:
                    change = True
            config.set(f"edmcoverlay2_{name}", val)
    if change:
        logger.info("Settings changes detected, restarting overlay")
        if edmcoverlay._edmcoverlay.stop_overlay():
            edmcoverlay._edmcoverlay.start_overlay()
