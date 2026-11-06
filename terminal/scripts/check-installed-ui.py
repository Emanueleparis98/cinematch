"""Exercise wheel-installed resources and all dialogs, outside the checkout."""
import asyncio
import tempfile
from pathlib import Path

from cinematch.app import CinemaApp
from cinematch.net import Network
from cinematch.prompts import catalog
from cinematch.screens import (
    AddonsScreen,
    CurrentsScreen,
    ImportScreen,
    PromptScreen,
    SettingsScreen,
)
from cinematch.service import Cinema
from cinematch.store import Store


async def main():
    with tempfile.TemporaryDirectory() as folder:
        service = Cinema(Network(), Store(Path(folder)))
        try:
            app = CinemaApp(service, autoload=False)
            async with app.run_test(size=(120, 40)) as pilot:
                await pilot.pause()
                assert len(catalog()) == 74
                for screen in [SettingsScreen(), AddonsScreen(), CurrentsScreen(), PromptScreen(), ImportScreen()]:
                    app.push_screen(screen)
                    await pilot.pause()
                    await pilot.press("escape")
                    await pilot.pause()
        finally:
            await service.close()


asyncio.run(main())
