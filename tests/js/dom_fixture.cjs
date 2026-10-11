const fs = require('node:fs');
const vm = require('node:vm');

// Small event/DOM harness for production vanilla-JS workflows. Drawing calls are
// captured as pixels/coordinates rather than comparing implementation strings.
function domFixture(ids = [], globals = {}) {
    const elements = new Map();
    const listeners = new Map();
    const frames = new Map();
    const alerts = [];
    let nextFrame = 0;
    class Event {
        constructor(type, options = {}) { this.type = type; Object.assign(this, options); }
        preventDefault() { this.defaultPrevented = true; }
    }
    class Element {
        constructor(tag = 'div', id = '') {
            this.tagName = tag.toUpperCase(); this.id = id; this.style = {}; this.dataset = {};
            this.children = []; this.value = ''; this.textContent = ''; this.innerText = '';
            this.events = new Map(); this.classes = new Set(); this.width = 256; this.height = 240;
            this.classList = {
                add: name => this.classes.add(name), remove: name => this.classes.delete(name),
                contains: name => this.classes.has(name),
                toggle: name => this.classes.has(name) ? this.classes.delete(name) : this.classes.add(name)
            };
            this.draws = [];
            this.attributes = new Map();
            this.context = new Proxy({
                fillRect: (...args) => this.draws.push({color: this.context.fillStyle, args}),
                createImageData: (w, h) => ({data: new Uint8ClampedArray(w * h * 4)}),
                putImageData: image => { this.lastImage = image; },
                getImageData: (x, y, w, h) => ({data: new Uint8ClampedArray(w * h * 4)})
            }, {get(target, key) { return key in target ? target[key] : () => {}; }});
            if (id) elements.set(id, this);
        }
        set innerHTML(value) { this.html = value; this.children = []; }
        get innerHTML() { return this.html || ''; }
        appendChild(child) { this.children.push(child); if (child.id) elements.set(child.id, child); return child; }
        append(...children) { children.forEach(child => this.appendChild(child)); }
        remove() { elements.delete(this.id); }
        click() { this.dispatchEvent(new Event('click')); this.onclick?.(new Event('click')); }
        addEventListener(type, fn) { const group = this.events.get(type) || []; group.push(fn); this.events.set(type, group); }
        removeEventListener(type, fn) { this.events.set(type, (this.events.get(type) || []).filter(value => value !== fn)); }
        dispatchEvent(event) { for (const fn of this.events.get(event.type) || []) fn(event); }
        querySelector() { return null; }
        querySelectorAll() { return []; }
        getContext() { return this.context; }
        getBoundingClientRect() { return {left: 0, top: 0, width: this.width, height: this.height}; }
        setAttribute(name, value) { this.attributes.set(name, String(value)); }
        getAttribute(name) { return this.attributes.get(name) ?? null; }
    }
    const document = new Element();
    document.body = new Element('body');
    document.getElementById = id => elements.get(id) || null;
    document.createElement = tag => new Element(tag);
    document.createElementNS = (_, tag) => new Element(tag);
    const window = {
        location: {hash: ''},
        addEventListener(type, fn) { const group = listeners.get(type) || []; group.push(fn); listeners.set(type, group); },
        removeEventListener(type, fn) { listeners.set(type, (listeners.get(type) || []).filter(value => value !== fn)); },
        dispatchEvent(event) { for (const fn of listeners.get(event.type) || []) fn(event); }
    };
    for (const id of ids) new Element(id.includes('canvas') ? 'canvas' : 'div', id);
    const context = vm.createContext({
        console, document, window, Event, CustomEvent: Event, navigator: {},
        history: {pushState() {}}, alert: text => alerts.push(text), confirm: () => true,
        requestAnimationFrame(fn) { frames.set(++nextFrame, fn); return nextFrame; },
        cancelAnimationFrame(id) { frames.delete(id); }, ...globals
    });
    function load(filename, exports = []) {
        vm.runInContext(fs.readFileSync(filename, 'utf8') + exports.map(name => `\nthis.${name} = ${name};`).join(''), context);
    }
    return {context, document, window, elements, listeners, frames, alerts, Event, load};
}

function deferred() {
    let resolve, reject;
    const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
    return {promise, resolve, reject};
}

module.exports = {domFixture, deferred};
