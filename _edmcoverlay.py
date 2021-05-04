import logging
import secrets
from pathlib import Path
from subprocess import Popen, PIPE

from config import appname, config

plugin_name = Path(__file__).parent.name
logger = logging.getLogger(f"{appname}.{plugin_name}")

logger.debug("edmcoverlay CE: lib loaded")

import errno
import json
import re
import socket
import threading
import time
from functools import wraps

IS_PRETENDING_TO_BE_EDMCOVERLAY = True

overlay_process: Popen = None


def find_overlay_binary() -> Path:
    our_directory = Path(__file__).resolve().parent
    overlay_binary = our_directory / "target" / "release" / "edmcoverlay"
    if not overlay_binary.exists():
        plug.show_error("edmcoverlay unable to find overlay binary")
        raise RuntimeError("edmcoverlay: unable to find overlay binary")
    return overlay_binary


def start_overlay():
    global overlay_process
    if not overlay_process:
        logger.info("edmcoverlay CE: starting overlay")
        xpos = int(config.get("edmcoverlay2_xpos") or 0)
        ypos = int(config.get("edmcoverlay2_ypos") or 0)
        width = int(config.get("edmcoverlay2_width") or 1920)
        height = int(config.get("edmcoverlay2_height") or 1080)
        overlay_process = Popen([find_overlay_binary(), str(xpos), str(ypos), str(width), str(height)], stdout=PIPE)
        for _ in range(5):
            line = overlay_process.stdout.readline()
            if line.strip() == b"server: ready to accept connections":
                time.sleep(0.01)
                break
            else:
                logger.warning("edmcoverlay CE: unexpected message from server:", line.decode())
        else:
            raise RuntimeError("edmcoverlay CE: server failed to start up")
        return True
    else:
        logger.warning("edmcoverlay CE: not starting overlay, already running")


def stop_overlay():
    global overlay_process
    if overlay_process:
        logger.info("edmcoverlay CE: stopping overlay")
        overlay_process.terminate()
        overlay_process.communicate()
        overlay_process = None
        return True
    else:
        logger.warning("edmcoverlay CE: not stopping overlay, not started")


def ensure_overlay():
    global overlay_process
    if not overlay_process:
        start_overlay()
        return True


class Overlay:
    def __init__(self, server="127.0.0.1", port=5010) -> None:
        # self._token = secrets.token_hex(4)
        self._server = server
        self._port = port
        self._conn = None
        self._connected = False

    def _ensure_connected(self):
        ensure_overlay()
        if not self._connected:
            try:
                self._conn = socket.socket()
                self._conn.connect((self._server, self._port))
            except socket.error as e:
                if e.errno == errno.ECONNREFUSED:
                    logger.warning("edmcoverlay CE: conn refused")
                else:
                    raise
            else:
                self._connected = True

    def send_raw(self, msg, _retry=True):
        logger.debug("edmcoverlay CE: send_raw!")
        self._ensure_connected()
        try:
            self._conn.send(json.dumps(msg).encode("utf-8") + b"\n")
        except socket.error as e:
            logger.exception("send_raw failed")
            self._connected = False
            if _retry:
                logger.info("retrying...")
                self.send_raw(msg, _retry=False)
            else:
                logger.error("double fault, not retrying")
                raise

    def send_message(self, msgid, text, color, x, y, ttl=4, size="normal"):
        # return self._overlay.send_message(self._token + str(msgid), text, color, x, y, ttl=ttl, size=size)
        self.send_raw({
            "id": msgid,
            "color": color,
            "text": text,
            "size": size,
            "x": x,
            "y": y,
            "ttl": ttl,
        })

    def send_shape(self, shapeid, shape, color, fill, x, y, w, h, ttl):
        # return self._overlay.send_shape(self._token + shapeid, shape, color, fill, x, y, w, h, ttl)
        self.send_raw({
            "id": shapeid,
            "shape": shape,
            "color": color,
            "fill": fill,
            "x": x,
            "y": y,
            "w": w,
            "h": h,
            "ttl": ttl,
        })
