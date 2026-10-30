# `fetch-failed`

- Type: error
- Kind: `io` (the host failed to supply bytes; often transient)

## What happened

A range source of `@lamina/pdf` (`httpRangeSource`) couldn't get bytes it asked for: the request failed (network error, CORS refusal, DNS) or the server answered with a status other than `200` or `206`, or with a `206` whose `Content-Range` (or length) doesn't match the range asked for: a `Content-Range` the browser hides because CORS doesn't expose it looks the same. It is also raised when a `callbackSource`'s read returns a different number of bytes than it was asked for.

## Why

Opening a remote file range by range depends on the server: it must allow the `Range` request (and, across origins, expose `Content-Range`, `Content-Length` and `ETag` through CORS), and must not compress the response (`Content-Encoding` breaks byte ranges). lamina never retries on its own; the error says the bytes are not available.

## How to fix

Check that the URL is reachable from where the code runs and that the server sends `Access-Control-Allow-Origin`, `Access-Control-Allow-Headers: Range` and `Access-Control-Expose-Headers: Content-Range, Content-Length, ETag`. Serve PDFs without `Content-Encoding`. Retry the call yourself if the failure was transient.

## Example

```js
const doc = await Document.openRange(httpRangeSource(url)); // throws { code: "fetch-failed", data: { status: 403 } }
```

## Data fields

- `reason`: `network`, `status`, `content-range` or `length` (lamina's own words). The URL is never included.
- `status`: the HTTP status, when there was a response.
- `expected`, `got`: for `length`, the bytes asked for and the bytes returned.
