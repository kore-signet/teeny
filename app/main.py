from quart import Quart, request
from db import ShortenDB
import tldextract
import os

safelist = os.environ["SAFELIST"].split(";")

app = Quart(__name__)
db = ShortenDB(app,os.environ["DB_PATH"])

@app.route("/<token>")
async def redirected(token: str):
    res = await db.get_url(token)
    if res:
        return "", 302, {'Location': res}
    else:
        return "Not found", 404

@app.route("/submit",methods=["POST"])
async def submit():
    url = (await request.form).get("url",None)
    if not url:
        return "Please specify a url", 400
    else:
        if '.'.join(tldextract.extract(url)[1:3]) in safelist:
            res = await db.set_url(url)
            return res, 200
        else:
            return "url not allowed", 403

if __name__ == "__main__":
    app.run(debug=True)
