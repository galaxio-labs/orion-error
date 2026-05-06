(function () {
  function textOf(header) {
    const anchor = header.querySelector("a.header");
    return (anchor || header).textContent.trim();
  }

  function buildPageToc() {
    const main = document.querySelector("main");
    if (!main) {
      return;
    }

    const headers = Array.from(main.querySelectorAll("h2, h3")).filter(function (header) {
      return header.id && textOf(header);
    });

    if (headers.length < 4) {
      return;
    }

    const nav = document.createElement("nav");
    nav.className = "page-toc";
    nav.setAttribute("aria-label", "On this page");

    const title = document.createElement("div");
    title.className = "page-toc-title";
    title.textContent = "On this page";
    nav.appendChild(title);

    const list = document.createElement("ol");
    nav.appendChild(list);

    headers.forEach(function (header) {
      const item = document.createElement("li");
      item.className = "page-toc-" + header.tagName.toLowerCase();

      const link = document.createElement("a");
      link.href = "#" + header.id;
      link.textContent = textOf(header);

      item.appendChild(link);
      list.appendChild(item);
    });

    document.body.appendChild(nav);

    const links = Array.from(nav.querySelectorAll("a"));

    function placeToc() {
      const mainRect = main.getBoundingClientRect();
      nav.style.left = mainRect.right + 34 + "px";
    }

    function setActive() {
      let active = headers[0];
      for (const header of headers) {
        if (header.getBoundingClientRect().top <= 120) {
          active = header;
        } else {
          break;
        }
      }

      links.forEach(function (link) {
        link.classList.toggle("active", link.getAttribute("href") === "#" + active.id);
      });
    }

    placeToc();
    setActive();
    window.addEventListener("resize", placeToc);
    document.addEventListener("scroll", placeToc, { passive: true });
    document.addEventListener("scroll", setActive, { passive: true });
  }

  document.addEventListener("DOMContentLoaded", buildPageToc);
})();
