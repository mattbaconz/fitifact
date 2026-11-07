import { useEffect, useRef } from "react";

// Slow topographic contour lines behind the hero. Raw WebGL2, started after
// the page's load event, paused offscreen and in hidden tabs, one still frame
// under reduced motion. Without WebGL2 the canvas stays transparent.
const VERTEX = `#version 300 es
in vec2 p;
void main() { gl_Position = vec4(p, 0.0, 1.0); }`;

const FRAGMENT = `#version 300 es
precision mediump float;
uniform vec2 res;
uniform float time;
uniform vec3 ink;
uniform float strength;
out vec4 color;

float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float noise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1, 0)), f.x),
             mix(hash(i + vec2(0, 1)), hash(i + 1.0), f.x), f.y);
}
float fbm(vec2 p) {
  float v = 0.0, a = 0.5;
  for (int i = 0; i < 4; i++) { v += a * noise(p); p = p * 2.03 + 17.0; a *= 0.5; }
  return v;
}
void main() {
  vec2 uv = gl_FragCoord.xy / res.y;
  vec2 q = uv * 1.35 + vec2(time * 0.012, -time * 0.008);
  float height = fbm(q + 1.4 * fbm(q * 0.8 + time * 0.02));
  float k = height * 16.0;
  float line = 1.0 - min(abs(fract(k) - 0.5) / fwidth(k), 1.0);
  float major = step(0.5, fract(floor(k) * 0.25 + 0.01)) * 0.6 + 0.4;
  vec2 c = gl_FragCoord.xy / res;
  float mask = smoothstep(0.0, 0.35, c.y) * (0.35 + 0.65 * smoothstep(0.15, 0.85, c.x));
  float a = line * major * mask * strength;
  color = vec4(ink * a, a);
}`;

function compile(gl: WebGL2RenderingContext, type: number, source: string) {
  const shader = gl.createShader(type)!;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  return shader;
}

function inkColor(): [number, number, number, number] {
  const style = getComputedStyle(document.documentElement);
  const hex = style.getPropertyValue("--ink").trim().replace("#", "");
  const value = parseInt(hex.length === 3 ? hex.replace(/./g, "$&$&") : hex, 16);
  const light = parseInt(style.getPropertyValue("--bg").trim().slice(1, 3), 16) > 128;
  return [
    ((value >> 16) & 255) / 255,
    ((value >> 8) & 255) / 255,
    (value & 255) / 255,
    light ? 0.3 : 0.26,
  ];
}

export function HeroShader() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let stop = () => {};
    let timer = 0;

    const start = () => {
      const gl = canvas.getContext("webgl2", { antialias: false, alpha: true });
      if (!gl) return;
      const program = gl.createProgram()!;
      gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERTEX));
      gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, FRAGMENT));
      gl.linkProgram(program);
      // Compile off the main thread where the driver allows it, then wait for it.
      const parallel = gl.getExtension("KHR_parallel_shader_compile");
      const linked = () => {
        if (parallel && !gl.getProgramParameter(program, parallel.COMPLETION_STATUS_KHR)) {
          timer = window.setTimeout(linked, 100);
          return;
        }
        if (gl.getProgramParameter(program, gl.LINK_STATUS)) run(gl, program);
      };
      linked();
    };

    const run = (gl: WebGL2RenderingContext, program: WebGLProgram) => {
      gl.useProgram(program);
      gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
      gl.enableVertexAttribArray(0);
      gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
      const uniform = (name: string) => gl.getUniformLocation(program, name);
      const [res, time, ink, strength] = ["res", "time", "ink", "strength"].map(uniform);

      const still = matchMedia("(prefers-reduced-motion: reduce)");
      let colors = inkColor();
      let visible = true;
      let frame = 0;
      let last = 0;
      let elapsed = 40;

      const draw = () => {
        const scale = Math.min(devicePixelRatio, 1.5);
        const width = Math.round(canvas.clientWidth * scale);
        const height = Math.round(canvas.clientHeight * scale);
        if (canvas.width !== width || canvas.height !== height) {
          canvas.width = width;
          canvas.height = height;
          gl.viewport(0, 0, width, height);
        }
        gl.uniform2f(res, width, height);
        gl.uniform1f(time, elapsed);
        gl.uniform3f(ink, colors[0], colors[1], colors[2]);
        gl.uniform1f(strength, colors[3]);
        gl.drawArrays(gl.TRIANGLES, 0, 3);
        canvas.dataset.ready = "";
      };
      const tick = (now: number) => {
        frame = 0;
        // ponytail: fixed 30 fps cap; drop it if the field ever needs to feel faster
        if (now - last >= 33) {
          elapsed += Math.min(now - last, 100) / 1000;
          last = now;
          draw();
        }
        schedule();
      };
      const schedule = () => {
        if (!frame && visible && !document.hidden && !still.matches) {
          last ||= performance.now();
          frame = requestAnimationFrame(tick);
        }
      };
      const pause = () => {
        cancelAnimationFrame(frame);
        frame = 0;
        last = 0;
      };
      const refresh = () => {
        colors = inkColor();
        pause();
        draw();
        schedule();
      };

      const seen = new IntersectionObserver(([entry]) => {
        visible = entry.isIntersecting;
        if (visible) schedule();
        else pause();
      });
      seen.observe(canvas);
      const onVisibility = () => (document.hidden ? pause() : schedule());
      document.addEventListener("visibilitychange", onVisibility);
      still.addEventListener("change", refresh);
      const scheme = matchMedia("(prefers-color-scheme: light)");
      scheme.addEventListener("change", refresh);
      const theme = new MutationObserver(refresh);
      theme.observe(document.documentElement, { attributeFilter: ["data-theme"] });
      const resized = new ResizeObserver(() => draw());
      resized.observe(canvas);

      draw();
      schedule();
      stop = () => {
        pause();
        seen.disconnect();
        theme.disconnect();
        resized.disconnect();
        document.removeEventListener("visibilitychange", onVisibility);
        still.removeEventListener("change", refresh);
        scheme.removeEventListener("change", refresh);
      };
    };

    // After load and the engine warm-up have had their turn on the main thread.
    const later = () => {
      timer = window.setTimeout(start, 1200);
    };
    if (document.readyState === "complete") later();
    else window.addEventListener("load", later, { once: true });
    return () => {
      window.removeEventListener("load", later);
      clearTimeout(timer);
      stop();
    };
  }, []);

  return <canvas ref={canvasRef} className="hero-field" aria-hidden="true" />;
}
