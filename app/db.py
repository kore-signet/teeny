from typing import Union
from quart import Quart
import secrets
import lmdb
import hashlib

class ShortenDB:
    def __init__(self, app: Quart, path: str) -> None:
        self.init_app(app)
        self._env = path

    def init_app(self, app: Quart) -> None:
        app.before_serving(self._before_serving)
        app.after_serving(self._after_serving)

    async def _before_serving(self) -> None:
        self._env = lmdb.Environment(self._env)

    async def _after_serving(self) -> None:
        self._env.close()

    async def get_url(self,token: str) -> Union[str,None]:
        with self._env.begin() as txn:
            return txn.get(token.encode())

    async def set_url(self,url: str) -> str:
        with self._env.begin(write = True) as txn:
            url_hash = hashlib.sha256()
            url_hash.update(url.encode())
            url_hash = url_hash.digest()

            v = txn.get(url_hash)

            if v:
                return v
            else:
                token = secrets.token_urlsafe(7)
                txn.put(url_hash,token.encode())
                txn.put(token.encode(),url.encode())
                return token
