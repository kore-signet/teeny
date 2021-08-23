from quart import Quart, request
from db import ShortenDB
from quart_cors import cors
import tldextract
import os

safelist = os.environ["SAFELIST"].split(";")

app = Quart(__name__)
app = cors(app, allow_origin="*",allow_methods=["GET","POST"], expose_headers=["location"])

db = ShortenDB(app,os.environ["DB_PATH"])

@app.route("/<token>")
async def redirected(token: str):
    res = await db.get_url(token)
    if res:
        return "", 302, {'Location': res}
    else:
        return "Not found", 404

# mostly since the location header refuses to work with cors
@app.route("/lookup/<token>")
async def lookup(token):
    res = await db.get_url(token)
    if res:
        return res, 200
    else:
        return "Not found", 404

@app.route("/submit",methods=["POST"])
async def submit():
    url = (await request.form).get("url",None)
    if not url:
        return "Please specify a url", 400
    else:
        if '.'.join(tldextract.extract(url)[:3]) in safelist:
            res = await db.set_url(url)
            return res, 200
        else:
            return "url not allowed", 403

if __name__ == "__main__":
    app.run(debug=True)
