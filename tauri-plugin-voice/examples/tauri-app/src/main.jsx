import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import OverlayApp from './OverlayApp';
import './style.css';

// Route based on hash fragment: #overlay renders the overlay-only view.
const isOverlay = window.location.hash === '#overlay';

ReactDOM.createRoot(document.getElementById('app')).render(
  <React.StrictMode>
    {isOverlay ? <OverlayApp /> : <App />}
  </React.StrictMode>,
);
