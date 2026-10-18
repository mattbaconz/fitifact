/* global document, localStorage */
try{document.documentElement.dataset.theme=localStorage.getItem('fitifact.theme')||'system'}catch{document.documentElement.dataset.theme='system'}
