"""Small in-memory TTL cache with request coalescing."""

from __future__ import annotations

import asyncio
import copy
from collections.abc import Awaitable, Callable
from time import monotonic
from typing import TypeVar

T = TypeVar("T")


class AsyncTTLCache:
    """Cache async results for a short time and share identical in-flight work."""

    def __init__(self, ttl: float, maxsize: int = 256):
        self.ttl = ttl
        self.maxsize = maxsize
        self._values: dict[object, tuple[float, object]] = {}
        self._tasks: dict[object, asyncio.Task] = {}

    def _prune(self) -> None:
        if len(self._values) <= self.maxsize:
            return
        now = monotonic()
        self._values = {key: item for key, item in self._values.items() if item[0] > now}
        while len(self._values) > self.maxsize:
            self._values.pop(next(iter(self._values)))

    def _finished(self, key: object, task: asyncio.Task) -> None:
        if self._tasks.get(key) is task:
            self._tasks.pop(key, None)
        if task.cancelled() or task.exception() is not None:
            return
        result = task.result()
        if result is None or result == [] or result == {} or result == "":
            return
        self._values[key] = (monotonic() + self.ttl, copy.deepcopy(result))
        self._prune()

    async def get(self, key: object, factory: Callable[[], Awaitable[T]]) -> T:
        saved = self._values.get(key)
        if saved is not None and saved[0] > monotonic():
            return copy.deepcopy(saved[1])
        task = self._tasks.get(key)
        if task is None:
            task = asyncio.create_task(factory())
            self._tasks[key] = task
            task.add_done_callback(lambda done, key=key: self._finished(key, done))
        return copy.deepcopy(await asyncio.shield(task))

    def clear(self) -> None:
        # Drop cached values, but let already-running requests finish so a
        # caller waiting on one is never cancelled by an unrelated refresh.
        self._values.clear()
