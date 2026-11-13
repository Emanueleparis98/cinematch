"""Exercise wheel-installed resources and all dialogs, outside the checkout."""
import asyncio
import tempfile
from pathlib import Path

from termovies.app import CinemaApp
from termovies.net import Network
from termovies.prompts import catalog
from termovies.screens import (
    AddonsScreen,
    CurrentsScreen,
    HelpScreen,
    ImportScreen,
    PromptScreen,
    SettingsScreen,
)
from termovies.service import Cinema
from termovies.store import Store


async def main():
    with tempfile.TemporaryDirectory() as folder:
        service = Cinema(Network(), Store(Path(folder)))
        try:
            app = CinemaApp(service, autoload=False)
            async with app.run_test(size=(120, 40)) as pilot:
                await pilot.pause()
                assert len(catalog()) == 74
                for screen in [SettingsScreen(), AddonsScreen(), CurrentsScreen(), PromptScreen(), ImportScreen(), HelpScreen()]:
                    app.push_screen(screen)
                    await pilot.pause()
                    await pilot.press("escape")
                    await pilot.pause()
        finally:
            await service.close()


asyncio.run(main())
