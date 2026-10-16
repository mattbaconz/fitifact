/* global document, localStorage */
try{document.documentElement.dataset.theme=localStorage.getItem('fitifact.theme')||'dark'}catch{document.documentElement.dataset.theme='dark'}
