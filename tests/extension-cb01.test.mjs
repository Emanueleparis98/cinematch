import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {
  defaultEpisode,
  isAllowedMediaUrl,
  isCb01ArticleUrl,
  parseCb01Article,
  parseMixdropPage,
  parseStayOnlineResponse,
  publicMedia,
  unpackPacker
} from '../extensions/cinematch/cb01-resolver.js';

const movieHtml = `<!doctype html><html><head><title>The Mongoose [HD] (2026) Streaming - FILM GRATIS by CB01 OFFICIAL</title></head><body class="single">
<article><div class="sequex-featured-img"><img class="responsive-locandina" alt="The Mongoose [HD] (2026)" src="/wp-content/poster.jpg"></div>
<div class="ignore-css">
<table class="cbtable"><tr><td class="tableinside"><a href="https://stayonline.pro/l/SD1/" rel="nofollow">Mixdrop</a></td></tr></table>
<table class="cbtable"><tr><td class="tableinside"><a href="https://uprot.net/msf/SD2">Maxstream</a></td></tr></table>
<table class="cbtable"><tr><td class="tableinside"><a href="https://swzz.xyz/download">Download</a></td></tr></table>
<div id="fancy-tabs"><ul><li><a class="link-catch-all">Maxstream HD</a></li><li><a class="link-catch-all">Mixdrop HD</a></li></ul></div>
<div class="tabs-catch-all" data-src="https://uprot.net/mse/HD1"><iframe src="https://uprot.net/mse/HD1"></iframe></div>
<div class="tabs-catch-all" data-src="https://stayonline.pro/e/HD2/"><iframe src="https://stayonline.pro/e/HD2/"></iframe></div>
</div></article></body></html>`;

const tvHtml = `<!doctype html><html><head><title>Wild Cards – 3×01/02/03/04 – ITA Streaming - SERIE TV GRATIS by CB01 OFFICIAL</title></head><body class="single">
<article><div class="sequex-featured-img"><img class="responsive-locandina" alt="Wild Cards – 3×01/02/03/04 – ITA" src="/serietv/wp-content/poster.jpg"></div>
<div class="ignore-css"><table class="cbtable"><tr><td>
<div class="sp-wrap sp-wrap-default"><div class="sp-head">STAGIONE 1 - ITA - HD</div><div class="sp-body">
<p>1×01 – <a href="https://stayonline.pro/l/S1E1M/">Maxstream</a> – <a href="http://stayonline.pro/l/S1E1X/">Mixdrop</a></p>
<p>1×02 – <a href="https://stayonline.pro/l/S1E2M/">Maxstream</a> – <a href="http://stayonline.pro/l/S1E2X/">Mixdrop</a></p>
</div></div>
<div class="sp-wrap sp-wrap-default"><div class="sp-head">STAGIONE 3 - ITA - HD</div><div class="sp-body">
<p>3×01 – <a href="https://stayonline.pro/l/S3E1M/">Maxstream</a> – <a href="http://stayonline.pro/l/S3E1X/">Mixdrop</a></p>
</div></div>
</td></tr></table></div></article></body></html>`;

test('movie pages expose HD/SD player links and exclude download hosts', () => {
  const parsed = parseCb01Article(movieHtml, 'https://cb01one.org/the-mongoose-hd-2026/');
  assert.equal(parsed.kind, 'movie');
  assert.equal(parsed.title, 'The Mongoose');
  assert.equal(parsed.year, '2026');
  assert.equal(parsed.poster, 'https://cb01one.org/wp-content/poster.jpg');
  assert.equal(parsed.sourceCount, 4);
  assert.deepEqual(parsed.sources.map(source => [source.label, source.quality, new URL(source.url).hostname]), [
    ['Mixdrop', 'HD', 'stayonline.pro'],
    ['Maxstream', 'HD', 'uprot.net'],
    ['Mixdrop', 'SD', 'stayonline.pro'],
    ['Maxstream', 'SD', 'uprot.net']
  ]);
  assert.ok(parsed.sources.every(source => !source.url.includes('swzz.xyz')));
});

test('TV pages group episodes by season and prefer Mixdrop links', () => {
  const parsed = parseCb01Article(tvHtml, 'https://cb01one.org/serietv/wild-cards/');
  assert.equal(parsed.kind, 'tv');
  assert.equal(parsed.title, 'Wild Cards');
  assert.deepEqual(parsed.seasons.map(season => [season.season, season.episodes.length]), [[1, 2], [3, 1]]);
  const first = parsed.seasons[0].episodes[0];
  assert.equal(first.episode, 1);
  assert.equal(first.sources.length, 2);
  assert.equal(first.sources[0].label, 'Mixdrop');
  assert.equal(first.sources[0].quality, 'HD');
  assert.equal(first.sources[0].url, 'https://stayonline.pro/l/S1E1X/');
  assert.equal(first.sources[1].label, 'Maxstream');
  assert.equal(defaultEpisode(parsed).season, 3);
  const media = publicMedia(parsed);
  assert.equal(media.kind, 'tv');
  assert.equal(media.seasons[0].episodes[0].sources, undefined);
});

test('stayonline responses are validated', () => {
  assert.equal(parseStayOnlineResponse({status: 'success', data: {value: 'https://m1xdrop.net/e/test'}}), 'https://m1xdrop.net/e/test');
  assert.throws(() => parseStayOnlineResponse({status: 'error', message: 'nope'}), /nope/);
  assert.throws(() => parseStayOnlineResponse({status: 'success', data: {value: 'javascript:alert(1)'}}), /indirizzo/);
});

test('standard Mixdrop packer is decoded without evaluating it', () => {
  const packed = readFileSync(new URL('./fixtures/cb01-mixdrop-packed.txt', import.meta.url), 'utf8').trim();
  const decoded = unpackPacker(packed);
  assert.match(decoded, /MDCore\.wurl/);
  assert.match(decoded, /mxcontent\.net/);
});

test('Mixdrop pages yield a direct MP4 and reject unrelated hosts', () => {
  const plain = parseMixdropPage('<html><script>MDCore.wurl="//media.mxcontent.net/video.mp4?s=1";MDCore.vfile="movie.mp4";</script></html>', 'https://m1xdrop.net/e/test');
  assert.equal(plain.url, 'https://media.mxcontent.net/video.mp4?s=1');
  assert.equal(plain.kind, 'video');
  assert.equal(plain.filename, 'movie.mp4');
  assert.throws(() => parseMixdropPage(`<html><body><h1>Mixdrop test page</h1><script>MDCore.wurl="https://evil.example/video.mp4";</script></body></html>`, 'https://m1xdrop.net/e/test'), /non supportato/);
});

test('CB01 URLs and media URLs are constrained to their official hosts', () => {
  assert.equal(isCb01ArticleUrl('https://cb01one.org/the-mongoose-hd-2026/'), true);
  assert.equal(isCb01ArticleUrl('https://cb01one.org/serietv/wild-cards/'), true);
  assert.equal(isCb01ArticleUrl('https://cb01one.org/wp-admin/'), false);
  assert.equal(isCb01ArticleUrl('https://evil.example/the-mongoose-hd-2026/'), false);
  assert.equal(isAllowedMediaUrl('https://81bfqpx79.mxcontent.net/v2/film.mp4?s=1'), true);
  assert.equal(isAllowedMediaUrl('https://evil.example/film.mp4'), false);
});
